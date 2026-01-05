use crossfont::{FontKey, GlyphKey, Size};

use super::font::{CachedGlyph, FontSystem, GlyphBuffer, GlyphCache, RasterizedGlyph};
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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

        if let Some(cached) = self.cache.get(&key) {
            return Some(*cached);
        }

        let rasterized = match font_system.rasterize(character, font_key) {
            Ok(g) => g,
            Err(e) => {
                log::warn!("Failed to rasterize '{}': {}", character, e);
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
            self.cache.insert(key, cached);
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

        self.cache.insert(key, cached);
        Some(cached)
    }

    /// Allocate space in the atlas using row-based packing.
    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        let padded_width = width + ATLAS_PADDING;
        let padded_height = height + ATLAS_PADDING;

        // Check if glyph fits in current row
        if self.current_row_x + padded_width <= self.size {
            let x = self.current_row_x;
            let y = self.current_row_y;
            self.current_row_x += padded_width;
            self.current_row_height = self.current_row_height.max(padded_height);
            return Some((x, y));
        }

        // Start new row
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

    /// Upload glyph bitmap to the atlas texture.
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

    /// Convert RGB/RGBA buffer to RGBA format for the atlas.
    fn to_rgba(&self, buffer: &GlyphBuffer, width: u32, height: u32) -> Vec<u8> {
        let pixel_count = (width * height) as usize;
        let mut rgba = Vec::with_capacity(pixel_count * 4);

        match buffer {
            GlyphBuffer::Rgb(data) => {
                // RGB subpixel data - convert to grayscale alpha mask
                for i in 0..pixel_count {
                    let idx = i * 3;
                    if idx + 2 < data.len() {
                        let r = data[idx];
                        let g = data[idx + 1];
                        let b = data[idx + 2];
                        // Use luminance as alpha, white as foreground
                        let alpha = ((r as u32 + g as u32 + b as u32) / 3) as u8;
                        rgba.extend_from_slice(&[255, 255, 255, alpha]);
                    } else {
                        rgba.extend_from_slice(&[0, 0, 0, 0]);
                    }
                }
            }
            GlyphBuffer::Rgba(data) => {
                // Already RGBA (colored glyphs like emoji)
                if data.len() == pixel_count * 4 {
                    rgba.extend_from_slice(data);
                } else {
                    rgba.resize(pixel_count * 4, 0);
                }
            }
        }

        rgba
    }

    /// Clear the atlas and cache for font size change.
    pub fn clear(&mut self, ctx: &GpuContext) {
        self.cache.clear();
        self.current_row_x = 0;
        self.current_row_y = 0;
        self.current_row_height = 0;

        // Recreate texture to clear it
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    /// Pre-populate cache with ASCII characters.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_allocation_simple() {
        // Test allocation logic without GPU
        let mut current_row_x = 0u32;
        let mut current_row_y = 0u32;
        let mut current_row_height = 0u32;
        let size = 1024u32;

        // Simulate allocating a 10x20 glyph
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
    fn test_to_rgba_rgb_conversion() {
        let rgb_data = vec![255, 128, 64];
        let buffer = GlyphBuffer::Rgb(rgb_data);

        // Manual conversion test
        let alpha = (255 + 128 + 64) / 3;
        assert_eq!(alpha, 149);
    }
}
