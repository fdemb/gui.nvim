use std::collections::HashMap;

use ahash::RandomState;
use crossfont::{
    BitmapBuffer, Error as CrossfontError, FontDesc, FontKey, GlyphKey, Metrics, Rasterize,
    Rasterizer, Size, Slant, Style, Weight,
};

/// Font configuration with fallback chain.
pub struct FontConfig {
    pub family: String,
    pub size_pt: f32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size_pt: 14.0,
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
    font_size: Size,
    metrics: Metrics,
}

impl FontSystem {
    pub fn new(config: &FontConfig) -> Result<Self, FontError> {
        let mut rasterizer = Rasterizer::new()?;
        let size = Size::new(config.size_pt);

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

        Ok(Self {
            rasterizer,
            font_key,
            bold_key,
            italic_key,
            bold_italic_key,
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

        let glyph = self.rasterizer.get_glyph(glyph_key)?;

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
pub struct GlyphCache {
    cache: HashMap<GlyphKey, CachedGlyph, RandomState>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::default(),
        }
    }

    pub fn get(&self, key: &GlyphKey) -> Option<&CachedGlyph> {
        self.cache.get(key)
    }

    pub fn insert(&mut self, key: GlyphKey, glyph: CachedGlyph) {
        self.cache.insert(key, glyph);
    }

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

        cache.insert(key, glyph);
        assert!(cache.get(&key).is_some());

        cache.clear();
        assert!(cache.get(&key).is_none());
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
