mod atlas;
mod batch;
mod color;
mod context;
mod font;
mod grid_renderer;
mod pipeline;

pub use context::{GpuContext, GpuContextError};
pub use grid_renderer::GridRendererError;

use color::{u32_to_linear_rgba, DEFAULT_BG_COLOR, DEFAULT_FG_COLOR};
use grid_renderer::GridRenderer;
use pipeline::RenderPipeline;

use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::config::Config;
use crate::editor::EditorState;

pub struct Renderer {
    ctx: GpuContext,
    pipeline: RenderPipeline,
    grid_renderer: GridRenderer,
    atlas_bind_group: wgpu::BindGroup,
    default_bg: [f32; 4],
    default_fg: [f32; 4],
}

impl Renderer {
    pub async fn new(window: Arc<Window>, config: Config) -> Result<Self, RendererError> {
        let scale_factor = window.scale_factor();
        let ctx = GpuContext::new(window).await?;
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

        Ok(Self {
            ctx,
            pipeline,
            grid_renderer,
            atlas_bind_group,
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
        Ok(())
    }

    pub fn render(
        &mut self,
        state: &EditorState,
        x_offset: f32,
        y_offset: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        self.grid_renderer.prepare(
            &self.ctx,
            state,
            self.default_bg,
            self.default_fg,
            x_offset,
            y_offset,
        );

        self.atlas_bind_group = self.pipeline.create_atlas_bind_group(
            &self.ctx,
            self.grid_renderer.atlas().texture_view(),
            self.grid_renderer.atlas().sampler(),
        );

        let output = self.ctx.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

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
