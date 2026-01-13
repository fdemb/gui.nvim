//! Cache for shaped text runs.
//!
//! This cache stores the results of HarfBuzz text shaping to avoid redundant
//! shaping calls when the same text content appears in multiple frames.
//!
//! The cache uses a content-based hash key (text + style + font) so that
//! identical text runs at different screen positions share the same cache entry.
//! This is inspired by Ghostty's shaping cache design.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

use super::shaper::ShapedGlyph;
use super::Style;

/// Maximum number of entries in the shaping cache.
/// ~2048 entries should cover most terminal content.
const MAX_CACHE_ENTRIES: usize = 2048;

/// Key for the shaping cache, based on run content hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapingCacheKey(u64);

impl ShapingCacheKey {
    /// Create a cache key from text content and style.
    ///
    /// The key is position-independent: identical text with the same style
    /// will produce the same key regardless of where it appears on screen.
    pub fn new(text: &str, style: Style) -> Self {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash the text content
        text.hash(&mut hasher);

        // Hash the style (bold/italic affects shaping)
        style.hash(&mut hasher);

        Self(hasher.finish())
    }
}

/// Cached shaped glyphs for a text run.
#[derive(Debug, Clone)]
pub struct CachedShapedRun {
    /// The shaped glyphs, with positions relative to run start.
    pub glyphs: Vec<ShapedGlyph>,
}

/// LRU-style cache for shaped text runs.
///
/// Uses a simple strategy: when full, remove the oldest entries.
/// This is simpler than true LRU but effective for terminal workloads
/// where recent content is most likely to be reused.
pub struct ShapingCache {
    /// Map from content hash to shaped glyphs.
    entries: HashMap<ShapingCacheKey, CachedShapedRun>,
    /// Order of insertion for simple eviction (VecDeque for O(1) pop_front).
    insertion_order: VecDeque<ShapingCacheKey>,
}

impl ShapingCache {
    /// Create a new empty shaping cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(MAX_CACHE_ENTRIES),
            insertion_order: VecDeque::with_capacity(MAX_CACHE_ENTRIES),
        }
    }

    /// Get glyphs by key. Returns None on cache miss.
    pub fn get_glyphs(&self, key: ShapingCacheKey) -> Option<&[ShapedGlyph]> {
        self.entries.get(&key).map(|e| e.glyphs.as_slice())
    }

    /// Insert a shaped run into the cache.
    pub fn insert(&mut self, key: ShapingCacheKey, glyphs: Vec<ShapedGlyph>) {
        if let std::collections::hash_map::Entry::Occupied(mut e) = self.entries.entry(key) {
            e.insert(CachedShapedRun { glyphs });
            return;
        }
        self.insert_inner(key, glyphs);
    }

    /// Internal insert that handles eviction and assumes key is not present.
    fn insert_inner(&mut self, key: ShapingCacheKey, glyphs: Vec<ShapedGlyph>) {
        // Evict oldest entries if at capacity (O(1) with VecDeque)
        while self.entries.len() >= MAX_CACHE_ENTRIES {
            if let Some(old_key) = self.insertion_order.pop_front() {
                self.entries.remove(&old_key);
            } else {
                break;
            }
        }

        self.entries.insert(key, CachedShapedRun { glyphs });
        self.insertion_order.push_back(key);
    }

    /// Clear the entire cache.
    ///
    /// Should be called when fonts change, as cached shaping results
    /// become invalid.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.insertion_order.clear();
    }

    /// Returns the number of entries in the cache.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for ShapingCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::font::CollectionIndex;

    fn make_glyph(glyph_id: u32, x_advance: i32) -> ShapedGlyph {
        ShapedGlyph {
            glyph_id,
            cluster: 0,
            x_advance,
            y_advance: 0,
            x_offset: 0,
            y_offset: 0,
            font_index: CollectionIndex::primary(Style::Regular),
        }
    }

    #[test]
    fn test_cache_hit() {
        let mut cache = ShapingCache::new();

        let key = ShapingCacheKey::new("hello", Style::Regular);
        let glyphs = vec![make_glyph(1, 100), make_glyph(2, 100)];

        cache.insert(key, glyphs.clone());

        let result = cache.get_glyphs(key);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_cache_miss() {
        let cache = ShapingCache::new();

        let key = ShapingCacheKey::new("hello", Style::Regular);
        assert!(cache.get_glyphs(key).is_none());
    }

    #[test]
    fn test_different_styles_different_keys() {
        let key_regular = ShapingCacheKey::new("hello", Style::Regular);
        let key_bold = ShapingCacheKey::new("hello", Style::Bold);

        assert_ne!(key_regular, key_bold);
    }

    #[test]
    fn test_same_content_same_key() {
        let key1 = ShapingCacheKey::new("hello", Style::Regular);
        let key2 = ShapingCacheKey::new("hello", Style::Regular);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = ShapingCache::new();

        // Fill cache beyond capacity
        for i in 0..(MAX_CACHE_ENTRIES + 100) {
            let text = format!("text{}", i);
            let key = ShapingCacheKey::new(&text, Style::Regular);
            cache.insert(key, vec![make_glyph(i as u32, 100)]);
        }

        // Should have evicted oldest entries
        assert!(cache.len() <= MAX_CACHE_ENTRIES);

        // Oldest entries should be gone
        let old_key = ShapingCacheKey::new("text0", Style::Regular);
        assert!(cache.get_glyphs(old_key).is_none());

        // Newest entries should still be there
        let new_key =
            ShapingCacheKey::new(&format!("text{}", MAX_CACHE_ENTRIES + 99), Style::Regular);
        assert!(cache.get_glyphs(new_key).is_some());
    }

    #[test]
    fn test_clear() {
        let mut cache = ShapingCache::new();

        let key = ShapingCacheKey::new("hello", Style::Regular);
        cache.insert(key, vec![make_glyph(1, 100)]);

        assert_eq!(cache.len(), 1);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.get_glyphs(key).is_none());
    }
}
