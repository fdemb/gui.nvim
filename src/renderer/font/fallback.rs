use objc2_core_foundation::{CFRange, CFRetained, CFString, CGFloat};
use objc2_core_text::CTFont;

use super::face::Face;

use std::collections::HashMap;

const NERD_FONT_NAME: &str = "Symbols Nerd Font";

pub struct FallbackResolver {
    cache: HashMap<u32, Option<CachedFallback>>,
    nerd_font: Option<CFRetained<CTFont>>,
    base_font: CFRetained<CTFont>,
    size_px: f32,
}

#[derive(Clone)]
struct CachedFallback {
    font_name: String,
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

    fn load_nerd_font(size_px: f32) -> Option<CFRetained<CTFont>> {
        let cf_name = CFString::from_str(NERD_FONT_NAME);
        let font = unsafe { CTFont::with_name(&cf_name, size_px as CGFloat, std::ptr::null()) };

        let created_name = unsafe { font.family_name() };
        if created_name.to_string() == NERD_FONT_NAME {
            log::debug!("Loaded Nerd Font for fallback resolution");
            Some(font)
        } else {
            log::warn!("Could not load '{}' for fallback", NERD_FONT_NAME);
            None
        }
    }

    pub fn discover(&mut self, codepoint: u32) -> Option<Face> {
        if let Some(cached) = self.cache.get(&codepoint) {
            return cached.as_ref().and_then(|c| self.load_face_by_name(&c.font_name));
        }

        let result = self.discover_uncached(codepoint);

        let cache_entry = result.as_ref().map(|face| CachedFallback {
            font_name: face.family_name().unwrap_or_default(),
        });
        self.cache.insert(codepoint, cache_entry);

        result
    }

    fn discover_uncached(&self, codepoint: u32) -> Option<Face> {
        if Self::is_nerd_font_codepoint(codepoint) {
            if let Some(face) = self.try_nerd_font(codepoint) {
                return Some(face);
            }
        }

        self.discover_system_fallback(codepoint)
    }

    fn try_nerd_font(&self, codepoint: u32) -> Option<Face> {
        let nerd_font = self.nerd_font.as_ref()?;

        let face = Face::from_ct_font(nerd_font.clone(), self.size_px).ok()?;
        if face.has_codepoint(codepoint) {
            Some(face)
        } else {
            None
        }
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

    fn load_face_by_name(&self, name: &str) -> Option<Face> {
        let cf_name = CFString::from_str(name);
        let ct_font = unsafe { CTFont::with_name(&cf_name, self.size_px as CGFloat, std::ptr::null()) };
        Face::from_ct_font(ct_font, self.size_px).ok()
    }

    fn is_nerd_font_codepoint(codepoint: u32) -> bool {
        matches!(codepoint,
            // Pomicons
            0xE000..=0xE00A |
            // Powerline + Powerline Extra (combined ranges)
            0xE0A0..=0xE0D4 |
            // Font Awesome Extension
            0xE200..=0xE2A9 |
            // Weather
            0xE300..=0xE3E3 |
            // Seti-UI + Custom
            0xE5FA..=0xE6B1 |
            // Devicons
            0xE700..=0xE7C5 |
            // Codicons
            0xEA60..=0xEBEB |
            // Font Awesome
            0xF000..=0xF2E0 |
            // Font Logos
            0xF300..=0xF32F |
            // Octicons
            0xF400..=0xF532 |
            // Material Design Icons (Supplementary Private Use Area)
            0xF0001..=0xF1AF0
        )
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
    fn test_nerd_font_codepoint_detection() {
        assert!(FallbackResolver::is_nerd_font_codepoint(0xE000));
        assert!(FallbackResolver::is_nerd_font_codepoint(0xE0A0));
        assert!(FallbackResolver::is_nerd_font_codepoint(0xF000));
        assert!(FallbackResolver::is_nerd_font_codepoint(0xF0001));

        assert!(!FallbackResolver::is_nerd_font_codepoint(0x0041)); // 'A'
        assert!(!FallbackResolver::is_nerd_font_codepoint(0x1F600)); // Emoji
    }

    #[test]
    fn test_fallback_resolver_creation() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let resolver = FallbackResolver::new(base_font, 14.0);
        assert!(resolver.cache.is_empty());
    }

    #[test]
    fn test_fallback_resolver_emoji() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        let result = resolver.discover('😀' as u32);
        assert!(result.is_some(), "Should find fallback for emoji");

        assert!(resolver.cache.contains_key(&('😀' as u32)));
    }

    #[test]
    fn test_fallback_resolver_ascii() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        let result = resolver.discover('A' as u32);
        assert!(result.is_some(), "Should find fallback for ASCII");
    }

    #[test]
    fn test_fallback_resolver_nerd_font() {
        crate::font_loader::register_embedded_fonts();

        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        let nerd_icon = 0xE62B;
        let result = resolver.discover(nerd_icon);

        if resolver.nerd_font.is_some() {
            assert!(result.is_some(), "Should find Nerd Font fallback for icon");
        }
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
