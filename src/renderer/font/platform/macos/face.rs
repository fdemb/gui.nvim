use objc2_core_foundation::{CFRange, CFRetained, CFString, CGFloat, CGPoint, CGRect};
use objc2_core_graphics::{CGBitmapInfo, CGColorSpace, CGGlyph};
use objc2_core_text::{CTFont, CTFontOrientation, CTFontSymbolicTraits};

use std::ptr::{self, NonNull};

use crate::renderer::font::{
    FaceError, FaceMetrics, FontFace, GlyphBuffer, HbFontWrapper, RasterizedGlyph,
};

mod hb_coretext_ffi {
    use std::ffi::c_void;

    extern "C" {
        pub fn hb_coretext_font_create(ct_font: *const c_void) -> *mut harfbuzz_sys::hb_font_t;
    }
}

fn hb_font_from_ct_font(ct_font: &CTFont, size_px: f32) -> Option<HbFontWrapper> {
    let ct_font_ptr = ct_font as *const CTFont as *const std::ffi::c_void;
    let hb_font = unsafe { hb_coretext_ffi::hb_coretext_font_create(ct_font_ptr) };
    if hb_font.is_null() {
        return None;
    }
    let scale = (size_px * 64.0) as i32;
    unsafe {
        harfbuzz_sys::hb_font_set_scale(hb_font, scale, scale);
    }
    unsafe { HbFontWrapper::from_raw(hb_font) }
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
        let ct_font = self.ct_font.clone();
        let hb_font = hb_font_from_ct_font(&ct_font, self.size_px)
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

    pub fn from_bytes(data: &'static [u8], size_px: f32) -> Result<Self, FaceError> {
        use objc2_core_foundation::CFData;
        use objc2_core_text::CTFontDescriptor;
        use objc2_core_text::CTFontManagerCreateFontDescriptorFromData;

        let cf_data = CFData::from_static_bytes(data);

        let descriptor: CFRetained<CTFontDescriptor> =
            unsafe { CTFontManagerCreateFontDescriptorFromData(&cf_data) }
                .ok_or(FaceError::TableCopyFailed)?;

        let ct_font =
            unsafe { CTFont::with_font_descriptor(&descriptor, size_px as CGFloat, ptr::null()) };

        Self::from_ct_font(ct_font, size_px)
    }

    pub fn create_style_variant(
        &self,
        style: crate::renderer::font::collection::Style,
    ) -> Option<Self> {
        let traits = match style {
            crate::renderer::font::collection::Style::Regular => return Some(self.clone()),
            crate::renderer::font::collection::Style::Bold => CTFontSymbolicTraits::TraitBold,
            crate::renderer::font::collection::Style::Italic => CTFontSymbolicTraits::TraitItalic,
            crate::renderer::font::collection::Style::BoldItalic => {
                CTFontSymbolicTraits::TraitBold | CTFontSymbolicTraits::TraitItalic
            }
        };

        let variant_ct_font = unsafe {
            self.ct_font.copy_with_symbolic_traits(
                self.size_px as CGFloat,
                std::ptr::null(),
                traits,
                traits,
            )
        }?;

        Self::from_ct_font(variant_ct_font, self.size_px).ok()
    }

    pub fn from_ct_font(ct_font: CFRetained<CTFont>, size_px: f32) -> Result<Self, FaceError> {
        let hb_font =
            hb_font_from_ct_font(&ct_font, size_px).ok_or(FaceError::HarfBuzzFaceCreation)?;
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

    #[allow(dead_code)]
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

        let bearing_x = rect.origin.x.floor() as i32;
        let bearing_y = (rect.origin.y + rect.size.height).floor() as i32;

        let frac_x = rect.origin.x - rect.origin.x.floor();
        let frac_y = rect.origin.y - rect.origin.y.floor();

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

        CGContext::set_allows_font_smoothing(Some(&context), true);
        CGContext::set_should_smooth_fonts(Some(&context), true);

        CGContext::set_allows_font_subpixel_positioning(Some(&context), true);
        CGContext::set_should_subpixel_position_fonts(Some(&context), true);

        CGContext::set_allows_font_subpixel_quantization(Some(&context), false);
        CGContext::set_should_subpixel_quantize_fonts(Some(&context), false);

        if is_color {
            CGContext::set_rgb_fill_color(Some(&context), 1.0, 1.0, 1.0, 1.0);
        } else {
            CGContext::set_gray_fill_color(Some(&context), 1.0, 1.0);
        }

        CGContext::translate_ctm(Some(&context), frac_x, frac_y);

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

impl FontFace for Face {
    fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    fn size_px(&self) -> f32 {
        self.size_px
    }

    fn has_codepoint(&self, codepoint: u32) -> bool {
        self.has_codepoint(codepoint)
    }

    fn glyph_index(&self, codepoint: u32) -> Option<u32> {
        self.glyph_index(codepoint)
    }

    fn render_glyph(&self, glyph_id: u32) -> Result<RasterizedGlyph, FaceError> {
        self.render_glyph(glyph_id)
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
