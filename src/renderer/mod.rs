mod atlas;
mod batch;
mod color;
mod context;
pub mod font;
mod geometry;
mod grid_renderer;
mod pipeline;

pub use context::{GpuContext, GpuContextError};
pub use grid_renderer::GridRendererError;

use color::{u32_to_linear_rgba, DEFAULT_BG_COLOR, DEFAULT_FG_COLOR};
use grid_renderer::{GridRenderer, RenderParams};
use pipeline::RenderPipeline;

use std::sync::Arc;

#[cfg(feature = "perf-stats")]
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::config::Config;
use crate::editor::EditorState;

pub struct Renderer {
    ctx: GpuContext,
    pipeline: RenderPipeline,
    grid_renderer: GridRenderer,
    atlas_bind_group: wgpu::BindGroup,
    /// Tracks atlas generation to avoid recreating the bind group every frame.
    atlas_bind_group_generation: u64,
    default_bg: [f32; 4],
    default_fg: [f32; 4],
}

impl Renderer {
    pub async fn new(window: Arc<Window>, config: Config) -> Result<Self, RendererError> {
        let scale_factor = window.scale_factor();
        let ctx = GpuContext::new(window, config.performance.vsync).await?;
        let grid_renderer = GridRenderer::new(&ctx, &config.font, scale_factor)?;
        let (cell_width, cell_height) = grid_renderer.cell_size();
        let pipeline = RenderPipeline::new(&ctx, cell_width, cell_height);

        let atlas_bind_group = pipeline.create_atlas_bind_group(
            &ctx,
            grid_renderer.atlas().texture_view(),
            grid_renderer.atlas().sampler(),
        );

        // Default colors in linear space
        let default_bg = u32_to_linear_rgba(DEFAULT_BG_COLOR);
        let default_fg = u32_to_linear_rgba(DEFAULT_FG_COLOR);

        let atlas_bind_group_generation = grid_renderer.atlas().generation();

        Ok(Self {
            ctx,
            pipeline,
            grid_renderer,
            atlas_bind_group,
            atlas_bind_group_generation,
            default_bg,
            default_fg,
        })
    }

    pub fn cell_size(&self) -> (f32, f32) {
        self.grid_renderer.cell_size()
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.ctx.resize(size);
            self.pipeline.resize(&self.ctx, size.width, size.height);
        }
    }

    pub fn update_default_colors(&mut self, fg: u32, bg: u32) {
        self.default_fg = u32_to_linear_rgba(fg);
        self.default_bg = u32_to_linear_rgba(bg);
    }

    pub fn update_font(
        &mut self,
        config: &crate::config::Config,
        scale_factor: f64,
    ) -> Result<(), RendererError> {
        self.grid_renderer
            .update_font(&self.ctx, &config.font, scale_factor)?;
        let (cell_width, cell_height) = self.grid_renderer.cell_size();
        self.pipeline
            .update_cell_size(&self.ctx, cell_width, cell_height);
        // Font change clears the atlas, so force bind group refresh
        self.sync_atlas_bind_group();
        Ok(())
    }

    /// Recreate the atlas bind group only when the atlas texture has changed
    /// (resize or clear), avoiding redundant GPU object creation every frame.
    fn sync_atlas_bind_group(&mut self) {
        let current_gen = self.grid_renderer.atlas().generation();
        if current_gen != self.atlas_bind_group_generation {
            self.atlas_bind_group = self.pipeline.create_atlas_bind_group(
                &self.ctx,
                self.grid_renderer.atlas().texture_view(),
                self.grid_renderer.atlas().sampler(),
            );
            self.atlas_bind_group_generation = current_gen;
        }
    }

    #[cfg(feature = "perf-stats")]
    pub fn render(
        &mut self,
        state: &EditorState,
        x_offset: f32,
        y_offset: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        let frame_start = Instant::now();

        // Phase 1: Prepare grid (batching, shaping, etc.)
        let prepare_start = Instant::now();
        let params = RenderParams::new(self.default_bg, self.default_fg, x_offset, y_offset);
        let prepare_stats = self.grid_renderer.prepare(&self.ctx, state, params);
        let prepare_duration = prepare_start.elapsed();

        // Phase 2: Recreate atlas bind group only if the atlas texture changed
        let bind_group_start = Instant::now();
        self.sync_atlas_bind_group();
        let bind_group_duration = bind_group_start.elapsed();

        // Phase 3: Get swap chain texture
        let swap_chain_start = Instant::now();
        let output = self.ctx.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let swap_chain_duration = swap_chain_start.elapsed();

        // Phase 4: Create command encoder and render pass
        let encode_start = Instant::now();
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.default_bg[0] as f64,
                            g: self.default_bg[1] as f64,
                            b: self.default_bg[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(self.pipeline.pipeline());
            render_pass.set_bind_group(0, self.pipeline.uniform_bind_group(), &[]);
            render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);

            let batcher = self.grid_renderer.batcher();

            if !batcher.backgrounds().is_empty() {
                render_pass.set_vertex_buffer(0, batcher.backgrounds().buffer().slice(..));
                render_pass.draw(0..6, 0..batcher.backgrounds().instance_count());
            }

            if !batcher.glyphs().is_empty() {
                render_pass.set_vertex_buffer(0, batcher.glyphs().buffer().slice(..));
                render_pass.draw(0..6, 0..batcher.glyphs().instance_count());
            }

            if !batcher.decorations().is_empty() {
                render_pass.set_vertex_buffer(0, batcher.decorations().buffer().slice(..));
                render_pass.draw(0..6, 0..batcher.decorations().instance_count());
            }
        }
        let encode_duration = encode_start.elapsed();

        // Phase 5: Submit and present
        let submit_start = Instant::now();
        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        let submit_duration = submit_start.elapsed();

        let frame_duration = frame_start.elapsed();

        // Log performance metrics
        let batcher = self.grid_renderer.batcher();
        log::debug!(
            "[PERF] Frame: {:>6.2}ms | prepare: {:>6.2}ms | bind_group: {:>6.2}ms | swap: {:>6.2}ms | encode: {:>6.2}ms | submit: {:>6.2}ms",
            frame_duration.as_secs_f64() * 1000.0,
            prepare_duration.as_secs_f64() * 1000.0,
            bind_group_duration.as_secs_f64() * 1000.0,
            swap_chain_duration.as_secs_f64() * 1000.0,
            encode_duration.as_secs_f64() * 1000.0,
            submit_duration.as_secs_f64() * 1000.0,
        );
        log::debug!(
            "[PERF] Batches: {} bg, {} glyphs, {} deco | Grid: {}x{} ({} cells)",
            batcher.backgrounds().instance_count(),
            batcher.glyphs().instance_count(),
            batcher.decorations().instance_count(),
            state.main_grid().width(),
            state.main_grid().height(),
            state.main_grid().width() * state.main_grid().height(),
        );
        log::debug!(
            "[PERF] Prepare breakdown: cells={}, runs={}, shape_calls={}, glyphs_shaped={}, glyph_cache={}/{}, shaping_cache={}/{}",
            prepare_stats.cells_processed,
            prepare_stats.runs_processed,
            prepare_stats.shape_calls,
            prepare_stats.glyphs_shaped,
            prepare_stats.glyph_cache_hits,
            prepare_stats.glyph_cache_hits + prepare_stats.glyph_cache_misses,
            prepare_stats.shaping_cache_hits,
            prepare_stats.shaping_cache_hits + prepare_stats.shaping_cache_misses,
        );
        log::debug!(
            "[PERF] Prepare timing: backgrounds={:.2}ms, shaping={:.2}ms, glyph_lookup={:.2}ms, batching={:.2}ms",
            prepare_stats.time_backgrounds.as_secs_f64() * 1000.0,
            prepare_stats.time_shaping.as_secs_f64() * 1000.0,
            prepare_stats.time_glyph_lookup.as_secs_f64() * 1000.0,
            prepare_stats.time_batching.as_secs_f64() * 1000.0,
        );

        Ok(())
    }

    #[cfg(not(feature = "perf-stats"))]
    pub fn render(
        &mut self,
        state: &EditorState,
        x_offset: f32,
        y_offset: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        // Phase 1: Prepare grid (batching, shaping, etc.)
        let params = RenderParams::new(self.default_bg, self.default_fg, x_offset, y_offset);
        self.grid_renderer.prepare(&self.ctx, state, params);

        // Phase 2: Recreate atlas bind group only if the atlas texture changed
        self.sync_atlas_bind_group();

        // Phase 3: Get swap chain texture
        let output = self.ctx.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Phase 4: Create command encoder and render pass
        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.default_bg[0] as f64,
                            g: self.default_bg[1] as f64,
                            b: self.default_bg[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(self.pipeline.pipeline());
            render_pass.set_bind_group(0, self.pipeline.uniform_bind_group(), &[]);
            render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);

            let batcher = self.grid_renderer.batcher();

            if !batcher.backgrounds().is_empty() {
                render_pass.set_vertex_buffer(0, batcher.backgrounds().buffer().slice(..));
                render_pass.draw(0..6, 0..batcher.backgrounds().instance_count());
            }

            if !batcher.glyphs().is_empty() {
                render_pass.set_vertex_buffer(0, batcher.glyphs().buffer().slice(..));
                render_pass.draw(0..6, 0..batcher.glyphs().instance_count());
            }

            if !batcher.decorations().is_empty() {
                render_pass.set_vertex_buffer(0, batcher.decorations().buffer().slice(..));
                render_pass.draw(0..6, 0..batcher.decorations().instance_count());
            }
        }

        // Phase 5: Submit and present
        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("GPU context error: {0}")]
    GpuContext(#[from] GpuContextError),

    #[error("Grid renderer error: {0}")]
    GridRenderer(#[from] GridRendererError),
}
