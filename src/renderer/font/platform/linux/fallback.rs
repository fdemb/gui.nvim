//! System fallback implementation for Linux using fontconfig.

use super::Face;
use crate::renderer::font::fallback::FallbackResolver;
use crate::renderer::font::traits::SystemFallback;

/// System fallback for Linux using fontconfig.
///
/// TODO: Implement using `fontconfig` crate to discover fallback fonts
/// that contain specific codepoints (emoji, international scripts, etc.).
pub struct LinuxSystemFallback;

impl SystemFallback<Face> for LinuxSystemFallback {
    fn new(_base_face: &Face, _size_px: f32) -> Self {
        Self
    }

    fn discover(&self, _codepoint: u32) -> Option<Face> {
        // TODO: Use fontconfig to find fallback font
        None
    }
}

/// Creates a FallbackResolver with system fallback only (no embedded nerd font).
pub fn create_fallback_resolver(_base_face: &Face) -> FallbackResolver<Face, LinuxSystemFallback> {
    let system_fallback = LinuxSystemFallback;
    FallbackResolver::new(system_fallback)
}

/// Creates a FallbackResolver with embedded nerd font support.
///
/// TODO: Load embedded nerd font when Linux font loading is implemented.
pub fn create_fallback_resolver_with_embedded(
    _base_face: &Face,
) -> Option<FallbackResolver<Face, LinuxSystemFallback>> {
    // Return None until Linux font loading is implemented
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_fallback_not_implemented() {
        let face = Face;
        let system_fallback = LinuxSystemFallback::new(&face, 14.0);

        assert!(system_fallback.discover('A' as u32).is_none());
        assert!(system_fallback.discover('😀' as u32).is_none());
    }

    #[test]
    fn test_create_fallback_resolver() {
        let face = Face;
        let resolver = create_fallback_resolver(&face);
        let _ = resolver;
    }
}
