#![allow(clippy::too_many_arguments)]

use super::pipeline::QuadInstance;
use super::GpuContext;

const INITIAL_BATCH_CAPACITY: usize = 65536;

/// Batch of quads for efficient GPU submission.
///
/// Grows the GPU buffer dynamically during `upload()` if more instances
/// were pushed than the current capacity allows.
pub struct QuadBatch {
    instances: Vec<QuadInstance>,
    buffer: wgpu::Buffer,
    capacity: usize,
}

impl QuadBatch {
    pub fn new(ctx: &GpuContext) -> Self {
        Self::with_capacity(ctx, INITIAL_BATCH_CAPACITY)
    }

    pub fn with_capacity(ctx: &GpuContext, capacity: usize) -> Self {
        let buffer = Self::create_buffer(ctx, capacity);

        Self {
            instances: Vec::with_capacity(capacity),
            buffer,
            capacity,
        }
    }

    fn create_buffer(ctx: &GpuContext, capacity: usize) -> wgpu::Buffer {
        let buffer_size = (capacity * std::mem::size_of::<QuadInstance>()) as u64;
        ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Add a background quad.
    pub fn push_background(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.instances
            .push(QuadInstance::background(x, y, width, height, color));
    }

    /// Add a glyph quad with atlas UV coordinates.
    pub fn push_glyph(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        uv_x: f32,
        uv_y: f32,
        uv_w: f32,
        uv_h: f32,
        color: [f32; 4],
        is_colored: bool,
    ) {
        self.instances.push(QuadInstance::glyph(
            x, y, width, height, uv_x, uv_y, uv_w, uv_h, color, is_colored,
        ));
    }

    /// Upload batch data to the GPU buffer, growing the buffer if needed.
    pub fn upload(&mut self, ctx: &GpuContext) {
        if self.instances.is_empty() {
            return;
        }

        if self.instances.len() > self.capacity {
            let new_capacity = self.instances.len().next_power_of_two();
            log::info!(
                "Batch buffer growing: {} -> {} instances",
                self.capacity,
                new_capacity
            );
            self.buffer = Self::create_buffer(ctx, new_capacity);
            self.capacity = new_capacity;
        }

        ctx.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.instances));
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    #[allow(dead_code)]
    pub fn vertex_count(&self) -> u32 {
        (self.instances.len() * 6) as u32
    }

    pub fn instance_count(&self) -> u32 {
        self.instances.len() as u32
    }
}

/// Batcher that manages separate batches for backgrounds, glyphs, and decorations.
pub struct RenderBatcher {
    backgrounds: QuadBatch,
    glyphs: QuadBatch,
    decorations: QuadBatch,
}

impl RenderBatcher {
    pub fn new(ctx: &GpuContext) -> Self {
        Self {
            backgrounds: QuadBatch::new(ctx),
            glyphs: QuadBatch::new(ctx),
            decorations: QuadBatch::new(ctx),
        }
    }

    pub fn clear(&mut self) {
        self.backgrounds.clear();
        self.glyphs.clear();
        self.decorations.clear();
    }

    pub fn push_background(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.backgrounds.push_background(x, y, width, height, color);
    }

    pub fn push_glyph(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        uv_x: f32,
        uv_y: f32,
        uv_w: f32,
        uv_h: f32,
        color: [f32; 4],
        is_colored: bool,
    ) {
        self.glyphs.push_glyph(
            x, y, width, height, uv_x, uv_y, uv_w, uv_h, color, is_colored,
        );
    }

    pub fn push_decoration(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.decorations.push_background(x, y, width, height, color);
    }

    pub fn upload(&mut self, ctx: &GpuContext) {
        self.backgrounds.upload(ctx);
        self.glyphs.upload(ctx);
        self.decorations.upload(ctx);
    }

    pub fn backgrounds(&self) -> &QuadBatch {
        &self.backgrounds
    }

    pub fn glyphs(&self) -> &QuadBatch {
        &self.glyphs
    }

    pub fn decorations(&self) -> &QuadBatch {
        &self.decorations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_batch_capacity() {
        // Should handle large grids: 200x100 = 20k cells, with glyphs + backgrounds + decorations
        assert!(INITIAL_BATCH_CAPACITY >= 65536);
    }

    #[test]
    fn test_quad_instance_memory_layout() {
        // Verify struct size matches GPU requirements (64 bytes aligned)
        assert_eq!(std::mem::size_of::<QuadInstance>(), 64);
    }
}
