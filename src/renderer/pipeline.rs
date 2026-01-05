use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::GpuContext;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    pub projection: [[f32; 4]; 4],
    pub screen_size: [f32; 2],
    pub cell_size: [f32; 2],
}

impl Uniforms {
    pub fn new(width: f32, height: f32, cell_width: f32, cell_height: f32) -> Self {
        Self {
            projection: Self::orthographic_projection(width, height),
            screen_size: [width, height],
            cell_size: [cell_width, cell_height],
        }
    }

    fn orthographic_projection(width: f32, height: f32) -> [[f32; 4]; 4] {
        [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, -2.0 / height, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ]
    }

    pub fn update(&mut self, width: f32, height: f32) {
        self.projection = Self::orthographic_projection(width, height);
        self.screen_size = [width, height];
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct QuadInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_offset: [f32; 2],
    pub uv_size: [f32; 2],
    pub color: [f32; 4],
    pub flags: u32,
    _padding: [u32; 3],
}

impl QuadInstance {
    pub const ATTRIBS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x2,  // size
        2 => Float32x2,  // uv_offset
        3 => Float32x2,  // uv_size
        4 => Float32x4,  // color
        5 => Uint32,     // flags
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn background(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self {
            position: [x, y],
            size: [width, height],
            uv_offset: [0.0, 0.0],
            uv_size: [0.0, 0.0],
            color,
            flags: 0,
            _padding: [0; 3],
        }
    }

    pub fn glyph(
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
    ) -> Self {
        let flags = FLAG_TEXTURED | if is_colored { FLAG_COLORED_GLYPH } else { 0 };
        Self {
            position: [x, y],
            size: [width, height],
            uv_offset: [uv_x, uv_y],
            uv_size: [uv_w, uv_h],
            color,
            flags,
            _padding: [0; 3],
        }
    }
}

pub const FLAG_TEXTURED: u32 = 1;
pub const FLAG_COLORED_GLYPH: u32 = 2;

pub struct RenderPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    atlas_bind_group_layout: wgpu::BindGroupLayout,
    uniforms: Uniforms,
}

impl RenderPipeline {
    pub fn new(ctx: &GpuContext, cell_width: f32, cell_height: f32) -> Self {
        let size = ctx.size();
        let uniforms = Uniforms::new(
            size.width as f32,
            size.height as f32,
            cell_width,
            cell_height,
        );

        let uniform_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let uniform_bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Uniform Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let uniform_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let atlas_bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Atlas Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &atlas_bind_group_layout],
                immediate_size: 0,
            });

        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Quad Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/quad.wgsl").into()),
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[QuadInstance::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.format(),
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            atlas_bind_group_layout,
            uniforms,
        }
    }

    pub fn resize(&mut self, ctx: &GpuContext, width: u32, height: u32) {
        self.uniforms.update(width as f32, height as f32);
        ctx.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    #[allow(dead_code)]
    pub fn update_cell_size(&mut self, ctx: &GpuContext, cell_width: f32, cell_height: f32) {
        self.uniforms.cell_size = [cell_width, cell_height];
        ctx.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn uniform_bind_group(&self) -> &wgpu::BindGroup {
        &self.uniform_bind_group
    }

    #[allow(dead_code)]
    pub fn atlas_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.atlas_bind_group_layout
    }

    pub fn create_atlas_bind_group(
        &self,
        ctx: &GpuContext,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atlas Bind Group"),
            layout: &self.atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniforms_orthographic_projection() {
        let uniforms = Uniforms::new(800.0, 600.0, 10.0, 20.0);
        let proj = uniforms.projection;

        // Top-left (0,0) should map to (-1, 1)
        let tl = [proj[0][0] * 0.0 + proj[3][0], proj[1][1] * 0.0 + proj[3][1]];
        assert!((tl[0] - (-1.0)).abs() < 0.001);
        assert!((tl[1] - 1.0).abs() < 0.001);

        // Bottom-right (800, 600) should map to (1, -1)
        let br = [
            proj[0][0] * 800.0 + proj[3][0],
            proj[1][1] * 600.0 + proj[3][1],
        ];
        assert!((br[0] - 1.0).abs() < 0.001);
        assert!((br[1] - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_quad_instance_size() {
        // Ensure struct is properly aligned for GPU
        assert_eq!(std::mem::size_of::<QuadInstance>(), 64);
    }

    #[test]
    fn test_quad_instance_background() {
        let quad = QuadInstance::background(10.0, 20.0, 100.0, 50.0, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(quad.position, [10.0, 20.0]);
        assert_eq!(quad.size, [100.0, 50.0]);
        assert_eq!(quad.flags, 0);
    }

    #[test]
    fn test_quad_instance_glyph() {
        let quad = QuadInstance::glyph(
            10.0,
            20.0,
            8.0,
            16.0,
            0.0,
            0.0,
            0.1,
            0.1,
            [1.0, 1.0, 1.0, 1.0],
            false,
        );
        assert_eq!(quad.flags, FLAG_TEXTURED);

        let colored = QuadInstance::glyph(
            10.0,
            20.0,
            8.0,
            16.0,
            0.0,
            0.0,
            0.1,
            0.1,
            [1.0, 1.0, 1.0, 1.0],
            true,
        );
        assert_eq!(colored.flags, FLAG_TEXTURED | FLAG_COLORED_GLYPH);
    }
}
