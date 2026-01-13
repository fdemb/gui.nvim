//! System fallback implementation for macOS using CoreText.

use objc2_core_foundation::{CFRange, CFRetained, CFString};
use objc2_core_text::CTFont;

use super::Face;
use crate::renderer::font::traits::SystemFallback;

/// System fallback for macOS using CoreText.
///
/// Uses `CTFont::for_string` to discover fallback fonts that contain
/// specific codepoints (emoji, international scripts, etc.).
pub struct CoreTextSystemFallback {
    base_font: CFRetained<CTFont>,
    size_px: f32,
}

impl SystemFallback<Face> for CoreTextSystemFallback {
    fn new(base_face: &Face, size_px: f32) -> Self {
        Self {
            base_font: base_face.ct_font().clone(),
            size_px,
        }
    }

    fn discover(&self, codepoint: u32) -> Option<Face> {
        let ch = char::from_u32(codepoint)?;
        let text = ch.to_string();
        let cf_string = CFString::from_str(&text);
        let range = CFRange::new(0, 1);

        let fallback_ct_font = unsafe { self.base_font.for_string(&cf_string, range) };

        let face = Face::from_ct_font(fallback_ct_font, self.size_px).ok()?;
        if face.has_codepoint(codepoint) {
            Some(face)
        } else {
            None
        }
    }
}

/// Helper function to create a FallbackResolver with nerd font and system fallback.
///
/// This is a convenience constructor for macOS that loads the embedded nerd font
/// and combines it with the CoreText system fallback.
pub fn create_fallback_resolver(
    base_face: &Face,
    nerd_font: Option<Face>,
) -> crate::renderer::font::fallback::FallbackResolver<Face, CoreTextSystemFallback> {
    let size_px = base_face.size_px();
    let system_fallback = CoreTextSystemFallback::new(base_face, size_px);

    if let Some(nerd_font) = nerd_font {
        crate::renderer::font::fallback::FallbackResolver::new(system_fallback)
            .with_nerd_font(nerd_font)
    } else {
        crate::renderer::font::fallback::FallbackResolver::new(system_fallback)
    }
}

/// Creates a FallbackResolver with the embedded nerd font loaded.
pub fn create_fallback_resolver_with_embedded(
    base_face: &Face,
) -> Option<crate::renderer::font::fallback::FallbackResolver<Face, CoreTextSystemFallback>> {
    use super::super::super::loader::EMBEDDED_NERD_FONT;
    use super::loader::create_font_from_bytes;

    let size_px = base_face.size_px();
    let ct_font = create_font_from_bytes(EMBEDDED_NERD_FONT, size_px)?;
    let nerd_font = Face::from_ct_font(ct_font, size_px).ok()?;

    log::info!("Loaded embedded Nerd Font: size={}px", size_px);

    Some(create_fallback_resolver(base_face, Some(nerd_font)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_fallback_emoji() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let base_face = Face::from_ct_font(base_font, 14.0).unwrap();

        let system_fallback = CoreTextSystemFallback::new(&base_face, 14.0);

        assert!(system_fallback.discover('😀' as u32).is_some());
    }

    #[test]
    fn test_system_fallback_ascii() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let base_face = Face::from_ct_font(base_font, 14.0).unwrap();

        let system_fallback = CoreTextSystemFallback::new(&base_face, 14.0);

        assert!(system_fallback.discover('A' as u32).is_some());
    }

    #[test]
    fn test_create_fallback_resolver() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let base_face = Face::from_ct_font(base_font, 14.0).unwrap();

        let resolver = create_fallback_resolver(&base_face, None);
        let _ = resolver;
    }

    #[test]
    fn test_create_fallback_resolver_with_embedded() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let base_face = Face::from_ct_font(base_font, 14.0).unwrap();

        let mut resolver = create_fallback_resolver_with_embedded(&base_face).unwrap();

        let nerd_icons = [0xE62B, 0xE0A0, 0xEF3E, 0xF001];
        for cp in nerd_icons {
            assert!(resolver.discover(cp).is_some(), "Should find 0x{:X}", cp);
        }
    }
}
