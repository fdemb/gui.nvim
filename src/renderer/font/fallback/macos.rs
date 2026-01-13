use objc2_core_foundation::{CFRange, CFRetained, CFString};
use objc2_core_text::CTFont;

use super::super::face::Face;
use super::super::loader::create_embedded_nerd_font;

use std::collections::HashMap;

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
                log::info!(
                    "Loaded embedded Nerd Font for fallback: size={}px, metrics={:?}",
                    size_px,
                    face.metrics()
                );
                Some(face)
            }
            Err(err) => {
                log::warn!("Failed to create Face from embedded Nerd Font: {}", err);
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
        if Self::is_nerd_font_codepoint(codepoint) {
            if let Some(face) = self.try_nerd_font(codepoint) {
                return Some(face);
            }
        }

        self.discover_system_fallback(codepoint)
    }

    fn try_nerd_font(&self, codepoint: u32) -> Option<Face> {
        let face = self.nerd_font.as_ref()?;
        if face.has_codepoint(codepoint) {
            Some(face.clone())
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

    fn is_nerd_font_codepoint(codepoint: u32) -> bool {
        matches!(codepoint,
            // Basic Multilingual Plane Private Use Area
            0xE000..=0xF8FF |
            // Supplementary Private Use Area-A
            0xF0000..=0xFFFFD |
            // Supplementary Private Use Area-B
            0x100000..=0x10FFFD
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
        // Known Nerd Font codepoints
        assert!(FallbackResolver::is_nerd_font_codepoint(0xE000)); // Pomicons start
        assert!(FallbackResolver::is_nerd_font_codepoint(0xE0A0)); // Powerline
        assert!(FallbackResolver::is_nerd_font_codepoint(0xEF3E)); // Gap that was previously missed
        assert!(FallbackResolver::is_nerd_font_codepoint(0xF000)); // Font Awesome
        assert!(FallbackResolver::is_nerd_font_codepoint(0xF0001)); // Material Design (Supplementary PUA)
        assert!(FallbackResolver::is_nerd_font_codepoint(0xF8FF)); // End of BMP PUA

        // NOT Nerd Font codepoints
        assert!(!FallbackResolver::is_nerd_font_codepoint(0x0041)); // 'A'
        assert!(!FallbackResolver::is_nerd_font_codepoint(0x1F600)); // Emoji (not in PUA)
        assert!(!FallbackResolver::is_nerd_font_codepoint(0xDFFF)); // Before PUA
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
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        assert!(
            resolver.nerd_font.is_some(),
            "Nerd font should be loaded from embedded data"
        );

        let nerd_icon = 0xE62B;
        let result = resolver.discover(nerd_icon);
        assert!(result.is_some(), "Should find Nerd Font fallback for icon");
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

    #[test]
    fn test_nerd_font_glyph_diagnostic() {
        let cf_name = CFString::from_str("Menlo");
        let base_font = unsafe { CTFont::with_name(&cf_name, 14.0, std::ptr::null()) };
        let mut resolver = FallbackResolver::new(base_font, 14.0);

        let test_codepoints: [(u32, &str); 4] = [
            (0xE62B, "Seti-UI"),
            (0xE0A0, "Powerline branch"),
            (0xF001, "Font Awesome"),
            (0xF0001, "Material Design"),
        ];

        for (cp, name) in test_codepoints {
            let is_nerd = FallbackResolver::is_nerd_font_codepoint(cp);
            let has_cp = resolver
                .nerd_font
                .as_ref()
                .map(|f| f.has_codepoint(cp))
                .unwrap_or(false);
            let discovered = resolver.discover(cp);

            println!(
                "{} (0x{:X}): is_nerd={}, has_cp={}, discovered={}",
                name,
                cp,
                is_nerd,
                has_cp,
                discovered.is_some()
            );

            // If it's a nerd font codepoint and the font has it, we should discover it
            if is_nerd && has_cp {
                assert!(
                    discovered.is_some(),
                    "Should discover {} (0x{:X})",
                    name,
                    cp
                );
            }
        }
    }
}
