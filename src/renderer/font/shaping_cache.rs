//! Cache for shaped text runs.
//!
//! This cache stores the results of HarfBuzz text shaping to avoid redundant
//! shaping calls when the same text content appears in multiple frames.
//!
//! The cache uses a content-based hash key (text + style + font) so that
//! identical text runs at different screen positions share the same cache entry.
//! This is inspired by Ghostty's shaping cache design.

use std::collections::HashMap;
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
    /// Order of insertion for simple eviction.
    insertion_order: Vec<ShapingCacheKey>,
    /// Statistics
    hits: u64,
    misses: u64,
}

impl ShapingCache {
    /// Create a new empty shaping cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(MAX_CACHE_ENTRIES),
            insertion_order: Vec::with_capacity(MAX_CACHE_ENTRIES),
            hits: 0,
            misses: 0,
        }
    }

    /// Look up a cached shaped run by key.
    pub fn get(&mut self, key: ShapingCacheKey) -> Option<&CachedShapedRun> {
        if let Some(entry) = self.entries.get(&key) {
            self.hits += 1;
            Some(entry)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a shaped run into the cache.
    pub fn insert(&mut self, key: ShapingCacheKey, glyphs: Vec<ShapedGlyph>) {
        // If already present, just update
        if self.entries.contains_key(&key) {
            self.entries.insert(key, CachedShapedRun { glyphs });
            return;
        }

        // Evict oldest entries if at capacity
        while self.entries.len() >= MAX_CACHE_ENTRIES {
            if let Some(old_key) = self.insertion_order.first().copied() {
                self.insertion_order.remove(0);
                self.entries.remove(&old_key);
            } else {
                break;
            }
        }

        // Insert new entry
        self.entries.insert(key, CachedShapedRun { glyphs });
        self.insertion_order.push(key);
    }

    /// Clear the entire cache.
    ///
    /// Should be called when fonts change, as cached shaping results
    /// become invalid.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.insertion_order.clear();
        // Don't reset stats - they're cumulative
    }

    /// Returns cache statistics.
    #[cfg(test)]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
        }
    }
}

#[cfg(test)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

#[cfg(test)]
impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
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

        let result = cache.get(key);
        assert!(result.is_some());
        assert_eq!(result.unwrap().glyphs.len(), 2);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ShapingCache::new();

        let key = ShapingCacheKey::new("hello", Style::Regular);
        let result = cache.get(key);

        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
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
        assert!(cache.entries.len() <= MAX_CACHE_ENTRIES);

        // Oldest entries should be gone
        let old_key = ShapingCacheKey::new("text0", Style::Regular);
        assert!(cache.get(old_key).is_none());

        // Newest entries should still be there
        let new_key =
            ShapingCacheKey::new(&format!("text{}", MAX_CACHE_ENTRIES + 99), Style::Regular);
        // Note: get() increments miss counter, so we check entries directly
        assert!(cache.entries.contains_key(&new_key));
    }

    #[test]
    fn test_clear() {
        let mut cache = ShapingCache::new();

        let key = ShapingCacheKey::new("hello", Style::Regular);
        cache.insert(key, vec![make_glyph(1, 100)]);

        assert_eq!(cache.entries.len(), 1);

        cache.clear();

        assert_eq!(cache.entries.len(), 0);
        assert!(cache.get(key).is_none());
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = ShapingCache::new();

        let key = ShapingCacheKey::new("hello", Style::Regular);
        cache.insert(key, vec![make_glyph(1, 100)]);

        // 1 miss (lookup before insert would have been a miss, but we didn't call get)
        cache.get(ShapingCacheKey::new("world", Style::Regular)); // miss
        cache.get(key); // hit
        cache.get(key); // hit

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 66.666).abs() < 1.0);
    }
}
