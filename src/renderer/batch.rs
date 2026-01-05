use super::pipeline::QuadInstance;
use super::GpuContext;

const MAX_BATCH_SIZE: usize = 65536;

/// Batch of quads for efficient GPU submission.
pub struct QuadBatch {
    instances: Vec<QuadInstance>,
    buffer: wgpu::Buffer,
    capacity: usize,
}

impl QuadBatch {
    pub fn new(ctx: &GpuContext) -> Self {
        Self::with_capacity(ctx, MAX_BATCH_SIZE)
    }

    pub fn with_capacity(ctx: &GpuContext, capacity: usize) -> Self {
        let buffer_size = (capacity * std::mem::size_of::<QuadInstance>()) as u64;
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            instances: Vec::with_capacity(capacity),
            buffer,
            capacity,
        }
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

    pub fn is_full(&self) -> bool {
        self.instances.len() >= self.capacity
    }

    /// Add a raw quad instance.
    pub fn push_instance(&mut self, instance: QuadInstance) {
        if self.is_full() {
            log::warn!("Batch full, cannot add instance");
            return;
        }
        self.instances.push(instance);
    }

    /// Add a background quad.
    pub fn push_background(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if self.is_full() {
            log::warn!("Batch full, cannot add background quad");
            return;
        }
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
        if self.is_full() {
            log::warn!("Batch full, cannot add glyph quad");
            return;
        }
        self.instances.push(QuadInstance::glyph(
            x, y, width, height, uv_x, uv_y, uv_w, uv_h, color, is_colored,
        ));
    }

    /// Upload batch data to GPU buffer.
    pub fn upload(&self, ctx: &GpuContext) {
        if self.instances.is_empty() {
            return;
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

    pub fn push_background_instance(&mut self, instance: QuadInstance) {
        self.backgrounds.push_instance(instance);
    }

    pub fn push_glyph_instance(&mut self, instance: QuadInstance) {
        self.glyphs.push_instance(instance);
    }

    pub fn push_decoration_instance(&mut self, instance: QuadInstance) {
        self.decorations.push_instance(instance);
    }

    pub fn upload(&self, ctx: &GpuContext) {
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
    fn test_quad_batch_capacity() {
        // Should handle large grids: 200x100 = 20k cells, with glyphs + backgrounds + decorations
        assert!(MAX_BATCH_SIZE >= 65536);
    }

    #[test]
    fn test_quad_instance_memory_layout() {
        // Verify struct size matches GPU requirements (64 bytes aligned)
        assert_eq!(std::mem::size_of::<QuadInstance>(), 64);
    }
}
