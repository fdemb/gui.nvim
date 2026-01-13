use objc2_core_foundation::{CFData, CFRetained, CGFloat};
use objc2_core_text::{CTFont, CTFontDescriptor, CTFontManagerCreateFontDescriptorFromData};

use super::EMBEDDED_NERD_FONT;

/// Creates a CTFont directly from embedded font data without global registration.
///
/// This is the modern approach (using `CTFontManagerCreateFontDescriptorFromData`)
/// rather than the deprecated `CTFontManagerRegisterGraphicsFont` which pollutes
/// the global font namespace.
pub fn create_embedded_nerd_font(size_px: f32) -> Option<CFRetained<CTFont>> {
    let cf_data = CFData::from_static_bytes(EMBEDDED_NERD_FONT);

    // Create font descriptor directly from data (modern, non-deprecated API)
    let descriptor: CFRetained<CTFontDescriptor> =
        unsafe { CTFontManagerCreateFontDescriptorFromData(&cf_data) }?;

    // Create font from descriptor
    let ct_font =
        unsafe { CTFont::with_font_descriptor(&descriptor, size_px as CGFloat, std::ptr::null()) };

    Some(ct_font)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_embedded_nerd_font() {
        let font = create_embedded_nerd_font(14.0);
        assert!(font.is_some(), "Should create embedded nerd font");

        let font = font.unwrap();
        let name = unsafe { font.family_name() };
        assert_eq!(
            name.to_string(),
            "Symbols Nerd Font",
            "Font family name should match"
        );
    }

    #[test]
    fn test_create_embedded_nerd_font_different_sizes() {
        for size in [10.0, 12.0, 14.0, 16.0, 24.0, 48.0] {
            let font = create_embedded_nerd_font(size);
            assert!(
                font.is_some(),
                "Should create embedded nerd font at size {}",
                size
            );
        }
    }
}
