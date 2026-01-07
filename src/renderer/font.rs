use std::collections::HashMap;

use ahash::RandomState;
use crossfont::{
    BitmapBuffer, Error as CrossfontError, FontDesc, FontKey, GlyphKey, Metrics, Rasterize,
    Rasterizer, Size, Slant, Style, Weight,
};

use crate::config::FontSettings;

/// Font configuration with fallback chain.
pub struct FontConfig {
    pub family: String,
    pub size_pt: f32,
    pub scale_factor: f32,
}

impl FontConfig {
    pub fn new(settings: &FontSettings, scale_factor: f64) -> Self {
        Self {
            family: settings.family.clone().unwrap_or_else(default_font_family),
            size_pt: settings.size.unwrap_or(14.0),
            scale_factor: scale_factor as f32,
        }
    }

    /// Returns the font size scaled for the display
    pub fn scaled_size(&self) -> f32 {
        self.size_pt * self.scale_factor
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size_pt: 14.0,
            scale_factor: 1.0,
        }
    }
}

/// Platform-specific default font family.
fn default_font_family() -> String {
    if cfg!(target_os = "macos") {
        "Menlo".to_string()
    } else if cfg!(windows) {
        "Consolas".to_string()
    } else {
        "monospace".to_string()
    }
}

/// Rasterized glyph with positioning data.
#[derive(Clone)]
pub struct RasterizedGlyph {
    #[allow(dead_code)]
    pub character: char,
    pub width: u32,
    pub height: u32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub buffer: GlyphBuffer,
}

#[derive(Clone)]
pub enum GlyphBuffer {
    Rgb(Vec<u8>),
    Rgba(Vec<u8>),
}

impl GlyphBuffer {
    pub fn is_colored(&self) -> bool {
        matches!(self, GlyphBuffer::Rgba(_))
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            GlyphBuffer::Rgb(b) | GlyphBuffer::Rgba(b) => b,
        }
    }
}

/// Font system with glyph rasterization support.
pub struct FontSystem {
    rasterizer: Rasterizer,
    font_key: FontKey,
    bold_key: FontKey,
    italic_key: FontKey,
    bold_italic_key: FontKey,
    symbols_key: Option<FontKey>,
    font_size: Size,
    metrics: Metrics,
}

impl FontSystem {
    pub fn new(config: &FontConfig) -> Result<Self, FontError> {
        let mut rasterizer = Rasterizer::new()?;
        // Use scaled size for HiDPI displays
        let size = Size::new(config.scaled_size());

        let regular_desc = FontDesc::new(
            &config.family,
            Style::Description {
                slant: Slant::Normal,
                weight: Weight::Normal,
            },
        );

        let font_key = match rasterizer.load_font(&regular_desc, size) {
            Ok(key) => key,
            Err(_) => {
                let fallback = FontDesc::new(
                    default_font_family(),
                    Style::Description {
                        slant: Slant::Normal,
                        weight: Weight::Normal,
                    },
                );
                rasterizer.load_font(&fallback, size)?
            }
        };

        // Load a glyph before querying metrics
        let glyph_key = GlyphKey {
            font_key,
            character: 'm',
            size,
        };
        rasterizer.get_glyph(glyph_key)?;

        let metrics = rasterizer.metrics(font_key, size)?;

        let bold_key = Self::load_variant(
            &mut rasterizer,
            &config.family,
            size,
            Weight::Bold,
            Slant::Normal,
        )
        .unwrap_or(font_key);
        let italic_key = Self::load_variant(
            &mut rasterizer,
            &config.family,
            size,
            Weight::Normal,
            Slant::Italic,
        )
        .unwrap_or(font_key);
        let bold_italic_key = Self::load_variant(
            &mut rasterizer,
            &config.family,
            size,
            Weight::Bold,
            Slant::Italic,
        )
        .unwrap_or(font_key);

        // Try to load the embedded Symbols Nerd Font
        let symbols_desc = FontDesc::new(
            "Symbols Nerd Font",
            Style::Description {
                slant: Slant::Normal,
                weight: Weight::Normal,
            },
        );
        let symbols_key = rasterizer.load_font(&symbols_desc, size).ok();

        Ok(Self {
            rasterizer,
            font_key,
            bold_key,
            italic_key,
            bold_italic_key,
            symbols_key,
            font_size: size,
            metrics,
        })
    }

    fn load_variant(
        rasterizer: &mut Rasterizer,
        family: &str,
        size: Size,
        weight: Weight,
        slant: Slant,
    ) -> Result<FontKey, FontError> {
        let desc = FontDesc::new(family, Style::Description { slant, weight });
        Ok(rasterizer.load_font(&desc, size)?)
    }

    #[allow(dead_code)]
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub fn cell_width(&self) -> f32 {
        self.metrics.average_advance as f32
    }

    pub fn cell_height(&self) -> f32 {
        self.metrics.line_height as f32
    }

    pub fn descent(&self) -> f32 {
        self.metrics.descent
    }

    pub fn underline_position(&self) -> f32 {
        self.metrics.underline_position
    }

    pub fn underline_thickness(&self) -> f32 {
        self.metrics.underline_thickness.max(1.0)
    }

    pub fn strikeout_position(&self) -> f32 {
        self.metrics.strikeout_position
    }

    pub fn strikeout_thickness(&self) -> f32 {
        self.metrics.strikeout_thickness.max(1.0)
    }

    pub fn font_key(&self) -> FontKey {
        self.font_key
    }

    pub fn font_key_for_style(&self, bold: bool, italic: bool) -> FontKey {
        match (bold, italic) {
            (false, false) => self.font_key,
            (true, false) => self.bold_key,
            (false, true) => self.italic_key,
            (true, true) => self.bold_italic_key,
        }
    }

    pub fn rasterize(
        &mut self,
        character: char,
        font_key: FontKey,
    ) -> Result<RasterizedGlyph, FontError> {
        let glyph_key = GlyphKey {
            font_key,
            character,
            size: self.font_size,
        };

        let glyph = match self.rasterizer.get_glyph(glyph_key) {
            Ok(g) => g,
            Err(e) => {
                // If glyph is missing and we have a symbols font, try that
                if let Some(symbols_key) = self.symbols_key {
                    if matches!(e, CrossfontError::MissingGlyph(_)) {
                        let symbol_glyph_key = GlyphKey {
                            font_key: symbols_key,
                            character,
                            size: self.font_size,
                        };
                        if let Ok(g) = self.rasterizer.get_glyph(symbol_glyph_key) {
                            // We found it in the symbols font!
                            // Note: we might need to adjust metrics or position if needed,
                            // but for now let's just use it.
                            g
                        } else {
                            // Still not found, return original error
                            return Err(FontError::Crossfont(e));
                        }
                    } else {
                        return Err(FontError::Crossfont(e));
                    }
                } else {
                    return Err(FontError::Crossfont(e));
                }
            }
        };

        let buffer = match glyph.buffer {
            BitmapBuffer::Rgb(data) => GlyphBuffer::Rgb(data),
            BitmapBuffer::Rgba(data) => GlyphBuffer::Rgba(data),
        };

        Ok(RasterizedGlyph {
            character,
            width: glyph.width.max(0) as u32,
            height: glyph.height.max(0) as u32,
            bearing_x: glyph.left,
            bearing_y: glyph.top,
            buffer,
        })
    }
}

/// Cached glyph with atlas coordinates.
#[derive(Clone, Copy, Debug)]
pub struct CachedGlyph {
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub width: u32,
    pub height: u32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub is_colored: bool,
}

/// Glyph cache keyed by character and font style.
/// Stores `Some(glyph)` for successful rasterizations, `None` for failed ones.
pub struct GlyphCache {
    cache: HashMap<GlyphKey, Option<CachedGlyph>, RandomState>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::default(),
        }
    }

    /// Returns `Some(Some(glyph))` if cached successfully, `Some(None)` if cached as failed,
    /// or `None` if not in cache at all.
    pub fn get(&self, key: &GlyphKey) -> Option<Option<&CachedGlyph>> {
        self.cache.get(key).map(|opt| opt.as_ref())
    }

    pub fn insert(&mut self, key: GlyphKey, glyph: Option<CachedGlyph>) {
        self.cache.insert(key, glyph);
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("Crossfont error: {0}")]
    Crossfont(#[from] CrossfontError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FontSettings;

    #[test]
    fn test_font_config_from_settings() {
        let settings = FontSettings {
            family: Some("Test Font".to_string()),
            size: Some(18.0),
        };
        let config = FontConfig::new(&settings, 2.0);

        assert_eq!(config.family, "Test Font");
        assert_eq!(config.size_pt, 18.0);
        assert_eq!(config.scale_factor, 2.0);
        assert_eq!(config.scaled_size(), 36.0);
    }

    #[test]
    fn test_font_config_defaults() {
        let settings = FontSettings::default();
        let config = FontConfig::new(&settings, 1.0);

        assert!(!config.family.is_empty()); // Should use platform default
        assert_eq!(config.size_pt, 14.0);
        assert_eq!(config.scale_factor, 1.0);
    }

    #[test]
    fn test_default_font_config() {
        let config = FontConfig::default();
        assert!(!config.family.is_empty());
        assert!(config.size_pt > 0.0);
    }

    #[test]
    fn test_font_system_creation() {
        let config = FontConfig::default();
        let font_system = FontSystem::new(&config);
        assert!(font_system.is_ok());
    }

    #[test]
    fn test_font_metrics() {
        let config = FontConfig::default();
        let font_system = FontSystem::new(&config).unwrap();

        assert!(font_system.cell_width() > 0.0);
        assert!(font_system.cell_height() > 0.0);
    }

    #[test]
    fn test_rasterize_ascii() {
        let config = FontConfig::default();
        let mut font_system = FontSystem::new(&config).unwrap();

        for c in 'a'..='z' {
            let result = font_system.rasterize(c, font_system.font_key());
            assert!(result.is_ok(), "Failed to rasterize '{}'", c);
        }
    }

    #[test]
    fn test_font_variants() {
        let config = FontConfig::default();
        let font_system = FontSystem::new(&config).unwrap();

        // Regular font should exist
        let regular = font_system.font_key_for_style(false, false);
        assert_eq!(regular, font_system.font_key);

        // Variants may fall back to regular, but should not panic
        let _ = font_system.font_key_for_style(true, false);
        let _ = font_system.font_key_for_style(false, true);
        let _ = font_system.font_key_for_style(true, true);
    }

    #[test]
    fn test_embedded_font_loading() {
        // Ensure embedded fonts are registered before testing loading
        #[cfg(target_os = "macos")]
        crate::font_loader::register_embedded_fonts();

        let config = FontConfig::default();
        let font_system = FontSystem::new(&config).unwrap();

        // On macOS, the embedded font should be registered and loaded.
        // On other platforms, it might not be available unless installed.
        if cfg!(target_os = "macos") {
            assert!(
                font_system.symbols_key.is_some(),
                "Embedded Symbols Nerd Font should be loaded on macOS"
            );
        }
    }

    #[test]
    fn test_glyph_cache() {
        use crossfont::Size;

        let mut cache = GlyphCache::new();
        let key = GlyphKey {
            font_key: FontKey::next(),
            character: 'a',
            size: Size::new(14.0),
        };
        let glyph = CachedGlyph {
            atlas_x: 0,
            atlas_y: 0,
            width: 10,
            height: 20,
            bearing_x: 0,
            bearing_y: 15,
            is_colored: false,
        };

        cache.insert(key, Some(glyph));
        assert!(cache.get(&key).is_some());
        assert!(cache.get(&key).unwrap().is_some());

        cache.clear();
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_glyph_cache_failure() {
        let mut cache = GlyphCache::new();
        let key = GlyphKey {
            font_key: FontKey::next(), // Generate a unique key
            character: '\u{f4d2}',     // A Nerd Font glyph that may not exist
            size: Size::new(14.0),
        };

        // Insert a cached failure (None)
        cache.insert(key, None);

        // Should return Some(None) - meaning "in cache, but failed"
        let result = cache.get(&key);
        assert!(result.is_some()); // It's in the cache
        assert!(result.unwrap().is_none()); // But marked as failed
    }

    #[test]
    fn test_glyph_buffer() {
        let rgb = GlyphBuffer::Rgb(vec![255, 0, 0]);
        assert!(!rgb.is_colored());
        assert_eq!(rgb.as_bytes(), &[255, 0, 0]);

        let rgba = GlyphBuffer::Rgba(vec![255, 0, 0, 255]);
        assert!(rgba.is_colored());
        assert_eq!(rgba.as_bytes(), &[255, 0, 0, 255]);
    }
}
