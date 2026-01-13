//! Font fallback resolution for macOS using CoreText.

use objc2_core_foundation::{CFRange, CFRetained, CFString};
use objc2_core_text::CTFont;

use super::super::face::Face;
use super::super::loader::create_embedded_nerd_font;

use std::collections::HashMap;

/// Resolves font fallbacks using a priority-ordered chain.
///
/// Fallback order:
/// 1. Embedded Nerd Font (icons/symbols)
/// 2. System fallback via CoreText (emoji, international scripts, etc.)
pub struct FallbackResolver {
    cache: HashMap<u32, Option<Face>>,
    nerd_font: Option<Face>,
    base_font: CFRetained<CTFont>,
    size_px: f32,
}

impl FallbackResolver {
    pub fn new(base_font: CFRetained<CTFont>, size_px: f32) -> Self {
        let nerd_font = Self::load_nerd_font(size_px);
        Self {
            cache: HashMap::new(),
            nerd_font,
            base_font,
            size_px,
        }
    }

    fn load_nerd_font(size_px: f32) -> Option<Face> {
        let ct_font = create_embedded_nerd_font(size_px)?;
        match Face::from_ct_font(ct_font, size_px) {
            Ok(face) => {
                log::info!("Loaded embedded Nerd Font: size={}px", size_px);
                Some(face)
            }
            Err(err) => {
                log::warn!("Failed to load embedded Nerd Font: {}", err);
                None
            }
        }
    }

    pub fn discover(&mut self, codepoint: u32) -> Option<Face> {
        if let Some(cached) = self.cache.get(&codepoint) {
            return cached.clone();
        }

        let result = self.discover_uncached(codepoint);
        self.cache.insert(codepoint, result.clone());
        result
    }

    fn discover_uncached(&self, codepoint: u32) -> Option<Face> {
        if let Some(ref face) = self.nerd_font {
            if face.has_codepoint(codepoint) {
                return Some(face.clone());
            }
        }

        self.discover_system_fallback(codepoint)
    }

    fn discover_system_fallback(&self, codepoint: u32) -> Option<Face> {
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

    #[allow(dead_code)]
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    #[allow(dead_code)]
    pub fn update_base_font(&mut self, base_font: CFRetained<CTFont>, size_px: f32) {
        self.base_font = base_font;
        if (self.size_px - size_px).abs() > 0.01 {
            self.size_px = size_px;
            self.nerd_font = Self::load_nerd_font(size_px);
        }
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_chain_creation() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let resolver = FallbackResolver::new(base_font, 14.0);

        assert!(resolver.cache.is_empty());
        assert!(resolver.nerd_font.is_some());
    }

    #[test]
    fn test_fallback_chain_nerd_font() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        let nerd_icons = [0xE62B, 0xE0A0, 0xEF3E, 0xF001];
        for cp in nerd_icons {
            assert!(resolver.discover(cp).is_some(), "Should find 0x{:X}", cp);
        }
    }

    #[test]
    fn test_fallback_chain_emoji() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        assert!(resolver.discover('😀' as u32).is_some());
    }

    #[test]
    fn test_fallback_chain_ascii() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        assert!(resolver.discover('A' as u32).is_some());
    }

    #[test]
    fn test_fallback_cache() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        let _ = resolver.discover('😀' as u32);
        assert!(resolver.cache.contains_key(&('😀' as u32)));

        resolver.clear_cache();
        assert!(resolver.cache.is_empty());
    }
}
