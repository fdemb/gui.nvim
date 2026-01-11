//! Linux font fallback implementation using fontconfig.
//!
//! TODO: Implement using:
//! - `fontconfig` crate with `FcFontMatch` / `FcFontSort` for fallback discovery
//! - Nerd Font detection via codepoint ranges (shared with macOS)

use super::super::face::Face;

use std::collections::HashMap;

pub struct FallbackResolver {
    cache: HashMap<u32, Option<Face>>,
    #[allow(dead_code)]
    size_px: f32,
}

impl FallbackResolver {
    pub fn new(size_px: f32) -> Self {
        Self {
            cache: HashMap::new(),
            size_px,
        }
    }

    pub fn discover(&mut self, codepoint: u32) -> Option<Face> {
        if let Some(cached) = self.cache.get(&codepoint) {
            return cached.clone();
        }

        // TODO: Use fontconfig to find a font containing this codepoint
        let result: Option<Face> = None;
        self.cache.insert(codepoint, result.clone());
        result
    }

    #[allow(dead_code)]
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
