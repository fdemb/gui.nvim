use std::collections::HashMap;

use ahash::RandomState;

use super::collection::CollectionIndex;

/// Cache key for shaped glyphs, using glyph ID instead of character.
///
/// This differs from the legacy cache which uses (character, font_key, size).
/// With HarfBuzz shaping, we get glyph IDs directly, and a single glyph can
/// represent multiple characters (ligatures) or multiple glyphs can represent
/// a single character (complex scripts).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GlyphCacheKey {
    /// The glyph ID from HarfBuzz shaping.
    pub glyph_id: u32,
    /// Index into the font collection (style + fallback index).
    pub font_index: CollectionIndex,
}

impl GlyphCacheKey {
    pub fn new(glyph_id: u32, font_index: CollectionIndex) -> Self {
        Self {
            glyph_id,
            font_index,
        }
    }
}

/// Cached glyph with atlas coordinates and rendering metadata.
#[derive(Clone, Copy, Debug)]
pub struct CachedGlyph {
    /// X position in the atlas texture.
    pub atlas_x: u32,
    /// Y position in the atlas texture.
    pub atlas_y: u32,
    /// Width of the glyph in pixels.
    pub width: u32,
    /// Height of the glyph in pixels.
    pub height: u32,
    /// Horizontal bearing (offset from origin to left edge).
    pub bearing_x: i32,
    /// Vertical bearing (offset from baseline to top edge).
    pub bearing_y: i32,
    /// Whether this is a color glyph (emoji).
    pub is_colored: bool,
}

impl CachedGlyph {
    /// Creates an empty cached glyph (for glyphs with no visual representation).
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            atlas_x: 0,
            atlas_y: 0,
            width: 0,
            height: 0,
            bearing_x: 0,
            bearing_y: 0,
            is_colored: false,
        }
    }
}

/// Glyph cache keyed by (glyph_id, font_index).
///
/// Stores `Some(glyph)` for successful rasterizations, `None` for failed ones.
/// Caching failures prevents repeated rasterization attempts for missing glyphs.
pub struct ShapedGlyphCache {
    cache: HashMap<GlyphCacheKey, Option<CachedGlyph>, RandomState>,
}

impl ShapedGlyphCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::default(),
        }
    }

    /// Returns `Some(Some(glyph))` if cached successfully, `Some(None)` if cached as failed,
    /// or `None` if not in cache at all.
    pub fn get(&self, key: &GlyphCacheKey) -> Option<Option<&CachedGlyph>> {
        self.cache.get(key).map(|opt| opt.as_ref())
    }

    /// Inserts a glyph into the cache.
    ///
    /// Pass `None` to cache a failed rasterization attempt.
    pub fn insert(&mut self, key: GlyphCacheKey, glyph: Option<CachedGlyph>) {
        self.cache.insert(key, glyph);
    }

    /// Checks if the cache contains an entry for the given key.
    #[allow(dead_code)]
    pub fn contains(&self, key: &GlyphCacheKey) -> bool {
        self.cache.contains_key(key)
    }

    /// Returns the number of entries in the cache.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the cache is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clears all entries from the cache.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for ShapedGlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::font::Style;

    #[test]
    fn test_glyph_cache_key_equality() {
        let key1 = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Regular));
        let key2 = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Regular));
        let key3 = GlyphCacheKey::new(43, CollectionIndex::primary(Style::Regular));
        let key4 = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Bold));

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key1, key4);
    }

    #[test]
    fn test_glyph_cache_key_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let key1 = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Regular));
        let key2 = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Regular));

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        key1.hash(&mut hasher1);
        key2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_cached_glyph_empty() {
        let glyph = CachedGlyph::empty();
        assert_eq!(glyph.width, 0);
        assert_eq!(glyph.height, 0);
        assert!(!glyph.is_colored);
    }

    #[test]
    fn test_shaped_glyph_cache_basic() {
        let mut cache = ShapedGlyphCache::new();
        let key = GlyphCacheKey::new(100, CollectionIndex::primary(Style::Regular));

        assert!(cache.get(&key).is_none());
        assert!(!cache.contains(&key));

        let glyph = CachedGlyph {
            atlas_x: 10,
            atlas_y: 20,
            width: 8,
            height: 16,
            bearing_x: 1,
            bearing_y: 14,
            is_colored: false,
        };

        cache.insert(key, Some(glyph));

        assert!(cache.contains(&key));
        let result = cache.get(&key);
        assert!(result.is_some());
        let cached = result.unwrap().unwrap();
        assert_eq!(cached.atlas_x, 10);
        assert_eq!(cached.atlas_y, 20);
        assert_eq!(cached.width, 8);
    }

    #[test]
    fn test_shaped_glyph_cache_failure() {
        let mut cache = ShapedGlyphCache::new();
        let key = GlyphCacheKey::new(0xFFFF, CollectionIndex::primary(Style::Regular));

        cache.insert(key, None);

        let result = cache.get(&key);
        assert!(result.is_some()); // Entry exists
        assert!(result.unwrap().is_none()); // But marked as failed
    }

    #[test]
    fn test_shaped_glyph_cache_clear() {
        let mut cache = ShapedGlyphCache::new();
        let key = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Regular));

        cache.insert(key, Some(CachedGlyph::empty()));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert!(cache.is_empty());
        assert!(!cache.contains(&key));
    }

    #[test]
    fn test_shaped_glyph_cache_multiple_entries() {
        let mut cache = ShapedGlyphCache::new();

        for i in 0..10 {
            let key = GlyphCacheKey::new(i, CollectionIndex::primary(Style::Regular));
            cache.insert(
                key,
                Some(CachedGlyph {
                    atlas_x: i * 10,
                    atlas_y: 0,
                    width: 8,
                    height: 16,
                    bearing_x: 0,
                    bearing_y: 14,
                    is_colored: false,
                }),
            );
        }

        assert_eq!(cache.len(), 10);

        for i in 0..10 {
            let key = GlyphCacheKey::new(i, CollectionIndex::primary(Style::Regular));
            let cached = cache.get(&key).unwrap().unwrap();
            assert_eq!(cached.atlas_x, i * 10);
        }
    }

    #[test]
    fn test_shaped_glyph_cache_different_styles() {
        let mut cache = ShapedGlyphCache::new();

        let glyph_id = 65; // 'A'
        let styles = [
            Style::Regular,
            Style::Bold,
            Style::Italic,
            Style::BoldItalic,
        ];

        for (i, style) in styles.iter().enumerate() {
            let key = GlyphCacheKey::new(glyph_id, CollectionIndex::primary(*style));
            cache.insert(
                key,
                Some(CachedGlyph {
                    atlas_x: i as u32 * 10,
                    atlas_y: 0,
                    width: 8,
                    height: 16,
                    bearing_x: 0,
                    bearing_y: 14,
                    is_colored: false,
                }),
            );
        }

        assert_eq!(cache.len(), 4);

        for (i, style) in styles.iter().enumerate() {
            let key = GlyphCacheKey::new(glyph_id, CollectionIndex::primary(*style));
            let cached = cache.get(&key).unwrap().unwrap();
            assert_eq!(cached.atlas_x, i as u32 * 10);
        }
    }

    #[test]
    fn test_shaped_glyph_cache_fallback_fonts() {
        let mut cache = ShapedGlyphCache::new();

        let glyph_id = 42;
        // Primary font (idx 0)
        let key1 = GlyphCacheKey::new(glyph_id, CollectionIndex::new(Style::Regular, 0));
        // First fallback (idx 1)
        let key2 = GlyphCacheKey::new(glyph_id, CollectionIndex::new(Style::Regular, 1));

        cache.insert(
            key1,
            Some(CachedGlyph {
                atlas_x: 0,
                atlas_y: 0,
                width: 8,
                height: 16,
                bearing_x: 0,
                bearing_y: 14,
                is_colored: false,
            }),
        );

        cache.insert(
            key2,
            Some(CachedGlyph {
                atlas_x: 10,
                atlas_y: 0,
                width: 10,
                height: 16,
                bearing_x: 0,
                bearing_y: 14,
                is_colored: true,
            }),
        );

        assert_eq!(cache.len(), 2);

        let cached1 = cache.get(&key1).unwrap().unwrap();
        let cached2 = cache.get(&key2).unwrap().unwrap();

        assert_eq!(cached1.atlas_x, 0);
        assert!(!cached1.is_colored);
        assert_eq!(cached2.atlas_x, 10);
        assert!(cached2.is_colored);
    }

    #[test]
    fn test_shaped_glyph_cache_default() {
        let cache = ShapedGlyphCache::default();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_glyph_cache_key_debug() {
        let key = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Bold));
        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("Bold"));
    }
}
