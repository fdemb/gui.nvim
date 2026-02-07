use crate::renderer::font::traits::{FontFace, SystemFallback};

use std::collections::HashMap;

/// Generic font fallback resolver using a priority-ordered chain.
///
/// Fallback order:
/// 1. Embedded Nerd Font (icons/symbols) - if loaded
/// 2. System fallback via `SystemFallback` trait implementation
pub struct FallbackResolver<F: FontFace, S: SystemFallback<F>> {
    cache: HashMap<u32, Option<F>>,
    nerd_font: Option<F>,
    system_fallback: S,
}

impl<F: FontFace + Clone, S: SystemFallback<F>> FallbackResolver<F, S> {
    /// Creates a new `FallbackResolver` with the given system fallback implementation.
    pub fn new(system_fallback: S) -> Self {
        Self {
            cache: HashMap::new(),
            nerd_font: None,
            system_fallback,
        }
    }

    /// Sets the embedded nerd font for icon/symbol fallback.
    pub fn with_nerd_font(mut self, nerd_font: F) -> Self {
        self.nerd_font = Some(nerd_font);
        self
    }

    /// Discovers a fallback face for the given codepoint.
    ///
    /// Returns `None` if no suitable fallback is found.
    pub fn discover(&mut self, codepoint: u32) -> Option<F> {
        if let Some(cached) = self.cache.get(&codepoint) {
            return cached.clone();
        }

        let result = self.discover_uncached(codepoint);
        self.cache.insert(codepoint, result.clone());
        result
    }

    /// Internal discovery without caching.
    fn discover_uncached(&self, codepoint: u32) -> Option<F> {
        if let Some(ref face) = self.nerd_font {
            if face.has_codepoint(codepoint) {
                return Some(face.clone());
            }
        }

        self.system_fallback.discover(codepoint)
    }

    /// Clears the fallback cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::font::{FaceError, FaceMetrics, HbFontWrapper, RasterizedGlyph};

    // Mock implementation for testing
    struct MockFace {
        codepoints: Vec<u32>,
        metrics: FaceMetrics,
    }

    impl FontFace for MockFace {
        fn metrics(&self) -> &FaceMetrics {
            &self.metrics
        }

        fn size_px(&self) -> f32 {
            14.0
        }

        fn has_codepoint(&self, codepoint: u32) -> bool {
            self.codepoints.contains(&codepoint)
        }

        fn glyph_index(&self, codepoint: u32) -> Option<u32> {
            if self.has_codepoint(codepoint) {
                Some(codepoint)
            } else {
                None
            }
        }

        fn render_glyph(&self, _glyph_id: u32) -> Result<RasterizedGlyph, FaceError> {
            Ok(RasterizedGlyph {
                character: '\0',
                width: 10,
                height: 10,
                bearing_x: 0,
                bearing_y: 10,
                buffer: crate::renderer::font::GlyphBuffer::Rgb(vec![0; 300]),
            })
        }

        fn hb_font(&self) -> &HbFontWrapper {
            unimplemented!("MockFace does not have a HarfBuzz font")
        }
    }

    impl Clone for MockFace {
        fn clone(&self) -> Self {
            Self {
                codepoints: self.codepoints.clone(),
                metrics: self.metrics,
            }
        }
    }

    struct MockSystemFallback {
        faces: Vec<MockFace>,
    }

    impl SystemFallback<MockFace> for MockSystemFallback {
        fn new(_base_face: &MockFace, _size_px: f32) -> Self {
            Self { faces: Vec::new() }
        }

        fn discover(&self, codepoint: u32) -> Option<MockFace> {
            for face in &self.faces {
                if face.has_codepoint(codepoint) {
                    return Some(face.clone());
                }
            }
            None
        }
    }

    #[test]
    fn test_fallback_resolver_creation() {
        let base_face = MockFace {
            codepoints: vec![],
            metrics: FaceMetrics::default(),
        };
        let system_fallback = MockSystemFallback::new(&base_face, 14.0);
        let resolver = FallbackResolver::new(system_fallback);

        assert!(resolver.cache.is_empty());
        assert!(resolver.nerd_font.is_none());
    }

    #[test]
    fn test_fallback_resolver_with_nerd_font() {
        let base_face = MockFace {
            codepoints: vec![],
            metrics: FaceMetrics::default(),
        };
        let system_fallback = MockSystemFallback::new(&base_face, 14.0);
        let nerd_font = MockFace {
            codepoints: vec![0xE62B],
            metrics: FaceMetrics::default(),
        };
        let mut resolver = FallbackResolver::new(system_fallback).with_nerd_font(nerd_font);

        assert!(resolver.nerd_font.is_some());
        assert!(resolver.discover(0xE62B).is_some());
    }

    #[test]
    fn test_fallback_cache() {
        let base_face = MockFace {
            codepoints: vec![],
            metrics: FaceMetrics::default(),
        };
        let system_fallback = MockSystemFallback::new(&base_face, 14.0);
        let nerd_font = MockFace {
            codepoints: vec![0xE62B],
            metrics: FaceMetrics::default(),
        };
        let mut resolver = FallbackResolver::new(system_fallback).with_nerd_font(nerd_font);

        let _ = resolver.discover(0xE62B);
        assert!(resolver.cache.contains_key(&0xE62B));

        resolver.clear_cache();
        assert!(resolver.cache.is_empty());
    }

    #[test]
    fn test_fallback_not_found() {
        let base_face = MockFace {
            codepoints: vec![],
            metrics: FaceMetrics::default(),
        };
        let system_fallback = MockSystemFallback::new(&base_face, 14.0);
        let mut resolver = FallbackResolver::new(system_fallback);

        assert!(resolver.discover(0x12345).is_none());
    }
}
