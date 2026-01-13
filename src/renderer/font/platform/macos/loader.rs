use objc2_core_foundation::{CFData, CFRetained, CGFloat};
use objc2_core_text::{CTFont, CTFontDescriptor, CTFontManagerCreateFontDescriptorFromData};

/// Creates a CTFont directly from embedded font data without global registration.
///
/// This is the modern approach (using `CTFontManagerCreateFontDescriptorFromData`)
/// rather than deprecated `CTFontManagerRegisterGraphicsFont` which pollutes
/// global font namespace.
pub fn create_font_from_bytes(data: &[u8], size_px: f32) -> Option<CFRetained<CTFont>> {
    let cf_data = CFData::from_bytes(data);

    // Create font descriptor directly from data (modern, non-deprecated API)
    let descriptor: CFRetained<CTFontDescriptor> =
        unsafe { CTFontManagerCreateFontDescriptorFromData(&cf_data) }?;

    // Create font from descriptor
    let ct_font =
        unsafe { CTFont::with_font_descriptor(&descriptor, size_px as CGFloat, std::ptr::null()) };

    Some(ct_font)
}
