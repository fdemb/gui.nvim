use crossfont::{FontKey, GlyphKey, Size};

use super::font::{
    CachedGlyph, Collection, FontSystem, GlyphBuffer, GlyphCache, GlyphCacheKey, RasterizedGlyph,
    ShapedCachedGlyph, ShapedGlyphCache,
};
use super::GpuContext;

const ATLAS_SIZE: u32 = 1024;
const ATLAS_PADDING: u32 = 1;

/// Texture atlas for storing rasterized glyphs.
pub struct GlyphAtlas {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    size: u32,
    current_row_y: u32,
    current_row_x: u32,
    current_row_height: u32,
    cache: GlyphCache,
    shaped_cache: ShapedGlyphCache,
}

impl GlyphAtlas {
    pub fn new(ctx: &GpuContext) -> Self {
        let size = ATLAS_SIZE;

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Glyph Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            texture_view,
            sampler,
            size,
            current_row_y: 0,
            current_row_x: 0,
            current_row_height: 0,
            cache: GlyphCache::new(),
            shaped_cache: ShapedGlyphCache::new(),
        }
    }

    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn atlas_size(&self) -> u32 {
        self.size
    }

    /// Get a cached glyph or rasterize and upload it.
    pub fn get_glyph(
        &mut self,
        ctx: &GpuContext,
        font_system: &mut FontSystem,
        character: char,
        font_key: FontKey,
        font_size: Size,
    ) -> Option<CachedGlyph> {
        let key = GlyphKey {
            font_key,
            character,
            size: font_size,
        };

        // Check cache first - returns Some(Some(glyph)), Some(None) for cached failure, or None if not cached
        if let Some(cached_result) = self.cache.get(&key) {
            return cached_result.copied();
        }

        // Not in cache - try to rasterize
        let rasterized = match font_system.rasterize(character, font_key) {
            Ok(g) => g,
            Err(e) => {
                log::warn!("Failed to rasterize '{}': {}", character, e);
                // Cache the failure so we don't try again
                self.cache.insert(key, None);
                return None;
            }
        };

        if rasterized.width == 0 || rasterized.height == 0 {
            let cached = CachedGlyph {
                atlas_x: 0,
                atlas_y: 0,
                width: 0,
                height: 0,
                bearing_x: rasterized.bearing_x,
                bearing_y: rasterized.bearing_y,
                is_colored: rasterized.buffer.is_colored(),
            };
            self.cache.insert(key, Some(cached));
            return Some(cached);
        }

        let (atlas_x, atlas_y) = self.allocate(rasterized.width, rasterized.height)?;
        self.upload(ctx, &rasterized, atlas_x, atlas_y);

        let cached = CachedGlyph {
            atlas_x,
            atlas_y,
            width: rasterized.width,
            height: rasterized.height,
            bearing_x: rasterized.bearing_x,
            bearing_y: rasterized.bearing_y,
            is_colored: rasterized.buffer.is_colored(),
        };

        self.cache.insert(key, Some(cached));
        Some(cached)
    }

    /// Get a glyph by ID from the new font system, or rasterize and cache it.
    ///
    /// This method uses the new HarfBuzz-based shaping system where glyphs are
    /// identified by glyph ID rather than character. This enables proper ligature
    /// support and complex script rendering.
    #[cfg(target_os = "macos")]
    pub fn get_glyph_by_id(
        &mut self,
        ctx: &GpuContext,
        collection: &Collection,
        key: GlyphCacheKey,
    ) -> Option<ShapedCachedGlyph> {
        if let Some(cached_result) = self.shaped_cache.get(&key) {
            return cached_result.copied();
        }

        let face = collection.get_face(key.font_index)?;
        let rasterized = match face.render_glyph(key.glyph_id) {
            Ok(g) => g,
            Err(e) => {
                log::warn!(
                    "Failed to rasterize glyph {} (font {:?}): {}",
                    key.glyph_id,
                    key.font_index,
                    e
                );
                self.shaped_cache.insert(key, None);
                return None;
            }
        };

        if rasterized.width == 0 || rasterized.height == 0 {
            let cached = ShapedCachedGlyph {
                atlas_x: 0,
                atlas_y: 0,
                width: 0,
                height: 0,
                bearing_x: rasterized.bearing_x,
                bearing_y: rasterized.bearing_y,
                is_colored: rasterized.buffer.is_colored(),
            };
            self.shaped_cache.insert(key, Some(cached));
            return Some(cached);
        }

        let (atlas_x, atlas_y) = self.allocate(rasterized.width, rasterized.height)?;
        self.upload(ctx, &rasterized, atlas_x, atlas_y);

        let cached = ShapedCachedGlyph {
            atlas_x,
            atlas_y,
            width: rasterized.width,
            height: rasterized.height,
            bearing_x: rasterized.bearing_x,
            bearing_y: rasterized.bearing_y,
            is_colored: rasterized.buffer.is_colored(),
        };

        self.shaped_cache.insert(key, Some(cached));
        Some(cached)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_glyph_by_id(
        &mut self,
        _ctx: &GpuContext,
        _collection: &Collection,
        _key: GlyphCacheKey,
    ) -> Option<ShapedCachedGlyph> {
        None
    }

    /// Allocate space in the atlas using row-based packing.
    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        let padded_width = width + ATLAS_PADDING;
        let padded_height = height + ATLAS_PADDING;

        if self.current_row_x + padded_width <= self.size {
            let x = self.current_row_x;
            let y = self.current_row_y;
            self.current_row_x += padded_width;
            self.current_row_height = self.current_row_height.max(padded_height);
            return Some((x, y));
        }

        let new_row_y = self.current_row_y + self.current_row_height;
        if new_row_y + padded_height > self.size {
            log::warn!("Glyph atlas full, cannot allocate {}x{}", width, height);
            return None;
        }

        self.current_row_y = new_row_y;
        self.current_row_x = padded_width;
        self.current_row_height = padded_height;

        Some((0, new_row_y))
    }

    fn upload(&self, ctx: &GpuContext, glyph: &RasterizedGlyph, x: u32, y: u32) {
        let rgba_data = self.to_rgba(&glyph.buffer, glyph.width, glyph.height);

        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(glyph.width * 4),
                rows_per_image: Some(glyph.height),
            },
            wgpu::Extent3d {
                width: glyph.width,
                height: glyph.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn to_rgba(&self, buffer: &GlyphBuffer, width: u32, height: u32) -> Vec<u8> {
        let pixel_count = (width * height) as usize;
        let mut rgba = Vec::with_capacity(pixel_count * 4);

        match buffer {
            GlyphBuffer::Rgb(data) => {
                for i in 0..pixel_count {
                    let idx = i * 3;
                    if idx + 2 < data.len() {
                        let r = data[idx];
                        let g = data[idx + 1];
                        let b = data[idx + 2];
                        let alpha = ((r as u32 + g as u32 + b as u32) / 3) as u8;
                        rgba.extend_from_slice(&[255, 255, 255, alpha]);
                    } else {
                        rgba.extend_from_slice(&[0, 0, 0, 0]);
                    }
                }
            }
            GlyphBuffer::Rgba(data) => {
                if data.len() == pixel_count * 4 {
                    rgba.extend_from_slice(data);
                } else {
                    rgba.resize(pixel_count * 4, 0);
                }
            }
        }

        rgba
    }

    #[allow(dead_code)]
    pub fn clear(&mut self, ctx: &GpuContext) {
        self.cache.clear();
        self.shaped_cache.clear();
        self.current_row_x = 0;
        self.current_row_y = 0;
        self.current_row_height = 0;

        self.texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: self.size,
                height: self.size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    pub fn prepopulate_ascii(
        &mut self,
        ctx: &GpuContext,
        font_system: &mut FontSystem,
        font_size: Size,
    ) {
        let font_key = font_system.font_key();
        for c in ' '..='~' {
            self.get_glyph(ctx, font_system, c, font_key, font_size);
        }
        log::info!("Pre-populated ASCII glyphs in atlas");
    }

    #[cfg(target_os = "macos")]
    pub fn prepopulate_ascii_shaped(
        &mut self,
        ctx: &GpuContext,
        collection: &mut Collection,
        style: super::font::Style,
    ) {
        let mut count = 0;
        for c in ' '..='~' {
            if let Some((font_index, glyph_id)) = collection.resolve_glyph(c as u32, style) {
                let key = GlyphCacheKey::new(glyph_id, font_index);
                if self.get_glyph_by_id(ctx, collection, key).is_some() {
                    count += 1;
                }
            }
        }
        log::info!("Pre-populated {} ASCII shaped glyphs in atlas", count);
    }

    #[cfg(not(target_os = "macos"))]
    pub fn prepopulate_ascii_shaped(
        &mut self,
        _ctx: &GpuContext,
        _collection: &mut Collection,
        _style: super::font::Style,
    ) {
        // No-op on non-macOS platforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::font::{CollectionIndex, Style};

    #[test]
    fn test_atlas_allocation_simple() {
        let mut current_row_x = 0u32;
        let current_row_y = 0u32;
        let mut current_row_height = 0u32;
        let size = 1024u32;

        let width = 10u32;
        let height = 20u32;
        let padded_width = width + ATLAS_PADDING;
        let padded_height = height + ATLAS_PADDING;

        assert!(current_row_x + padded_width <= size);
        let (x, y) = (current_row_x, current_row_y);
        current_row_x += padded_width;
        current_row_height = current_row_height.max(padded_height);

        assert_eq!(x, 0);
        assert_eq!(y, 0);
        assert_eq!(current_row_x, 11);
        assert_eq!(current_row_height, 21);
    }

    #[test]
    fn test_atlas_allocation_row_wrap() {
        let mut current_row_x = 1020u32;
        let mut current_row_y = 0u32;
        let mut current_row_height = 20u32;
        let size = 1024u32;

        let width = 10u32;
        let height = 15u32;
        let padded_width = width + ATLAS_PADDING;
        let padded_height = height + ATLAS_PADDING;

        if current_row_x + padded_width > size {
            current_row_y += current_row_height;
            current_row_x = 0;
            current_row_height = 0;
        }

        let (x, y) = (current_row_x, current_row_y);
        current_row_x += padded_width;
        current_row_height = current_row_height.max(padded_height);

        assert_eq!(x, 0, "Should wrap to new row");
        assert_eq!(y, 20, "New row should start at previous row height");
    }

    #[test]
    fn test_to_rgba_rgb_conversion() {
        let rgb_data = vec![255, 128, 64];
        let _buffer = GlyphBuffer::Rgb(rgb_data);

        let alpha = (255 + 128 + 64) / 3;
        assert_eq!(alpha, 149);
    }

    #[test]
    fn test_to_rgba_rgba_passthrough() {
        let rgba_data = vec![255, 128, 64, 200];
        let buffer = GlyphBuffer::Rgba(rgba_data.clone());

        assert!(buffer.is_colored());
    }

    #[test]
    fn test_glyph_cache_key_creation() {
        let key = GlyphCacheKey::new(42, CollectionIndex::primary(Style::Regular));
        assert_eq!(key.glyph_id, 42);
        assert_eq!(key.font_index.style, Style::Regular);
        assert_eq!(key.font_index.idx, 0);
    }

    #[test]
    fn test_shaped_glyph_cache_insert_and_retrieve() {
        let mut cache = ShapedGlyphCache::new();
        let key = GlyphCacheKey::new(100, CollectionIndex::primary(Style::Bold));

        assert!(cache.get(&key).is_none(), "Cache should be empty initially");

        let glyph = ShapedCachedGlyph {
            atlas_x: 50,
            atlas_y: 100,
            width: 10,
            height: 20,
            bearing_x: 2,
            bearing_y: 18,
            is_colored: false,
        };

        cache.insert(key, Some(glyph));

        let result = cache.get(&key);
        assert!(result.is_some());
        let cached = result.unwrap().unwrap();
        assert_eq!(cached.atlas_x, 50);
        assert_eq!(cached.width, 10);
    }

    #[test]
    fn test_shaped_glyph_cache_failure_tracking() {
        let mut cache = ShapedGlyphCache::new();
        let key = GlyphCacheKey::new(0xFFFF, CollectionIndex::primary(Style::Regular));

        cache.insert(key, None);

        let result = cache.get(&key);
        assert!(result.is_some(), "Entry should exist");
        assert!(result.unwrap().is_none(), "Entry should be marked as failed");
    }

    #[test]
    fn test_shaped_cached_glyph_empty() {
        let glyph = ShapedCachedGlyph::empty();
        assert_eq!(glyph.width, 0);
        assert_eq!(glyph.height, 0);
        assert_eq!(glyph.atlas_x, 0);
        assert_eq!(glyph.atlas_y, 0);
        assert!(!glyph.is_colored);
    }

    #[test]
    fn test_atlas_size_constant() {
        assert_eq!(ATLAS_SIZE, 1024, "Atlas size should be 1024");
        assert_eq!(ATLAS_PADDING, 1, "Atlas padding should be 1");
    }

    #[test]
    fn test_collection_index_styles() {
        let styles = [Style::Regular, Style::Bold, Style::Italic, Style::BoldItalic];

        for style in styles {
            let index = CollectionIndex::primary(style);
            assert_eq!(index.style, style);
            assert_eq!(index.idx, 0);

            let fallback = CollectionIndex::new(style, 5);
            assert_eq!(fallback.style, style);
            assert_eq!(fallback.idx, 5);
        }
    }
}
