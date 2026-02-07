//! System fallback implementation for Windows using DirectWrite.

use super::Face;
use crate::renderer::font::fallback::FallbackResolver;
use crate::renderer::font::traits::SystemFallback;

/// System fallback for Windows using DirectWrite.
///
/// TODO: Implement using `IDWriteFontFallback` to discover fallback fonts
/// that contain specific codepoints (emoji, international scripts, etc.).
pub struct WindowsSystemFallback;

impl SystemFallback<Face> for WindowsSystemFallback {
    fn new(_base_face: &Face, _size_px: f32) -> Self {
        Self
    }

    fn discover(&self, _codepoint: u32) -> Option<Face> {
        // TODO: Use IDWriteFontFallback to find fallback font
        None
    }
}

/// Creates a FallbackResolver, optionally with a nerd font for icon support.
pub fn create_fallback_resolver(
    _base_face: &Face,
    nerd_font: Option<Face>,
) -> FallbackResolver<Face, WindowsSystemFallback> {
    let system_fallback = WindowsSystemFallback;
    let resolver = FallbackResolver::new(system_fallback);
    if let Some(nerd_font) = nerd_font {
        resolver.with_nerd_font(nerd_font)
    } else {
        resolver
    }
}

/// Creates a FallbackResolver with embedded nerd font support.
///
/// TODO: Load embedded nerd font when Windows font loading is implemented.
pub fn create_fallback_resolver_with_embedded(
    _base_face: &Face,
) -> Option<FallbackResolver<Face, WindowsSystemFallback>> {
    // Return None until Windows font loading is implemented
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_fallback_not_implemented() {
        let face = Face;
        let system_fallback = WindowsSystemFallback::new(&face, 14.0);

        assert!(system_fallback.discover('A' as u32).is_none());
        assert!(system_fallback.discover('😀' as u32).is_none());
    }

    #[test]
    fn test_create_fallback_resolver() {
        let face = Face;
        let resolver = create_fallback_resolver(&face, None);
        let _ = resolver;
    }
}
