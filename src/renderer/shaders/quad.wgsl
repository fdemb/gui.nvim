// Unified quad shader for rendering cell backgrounds and text glyphs.
//
// Rendering is done via instanced quads. Each instance represents either:
// - A cell background (solid color, no texture)
// - A text glyph (textured, alpha-blended)
//
// The shader uses 6 vertices per quad (two triangles), with vertices
// generated procedurally from instance data.

struct Uniforms {
    // Orthographic projection matrix (screen coords to clip space)
    projection: mat4x4<f32>,
    // Screen size in pixels
    screen_size: vec2<f32>,
    // Cell dimensions in pixels
    cell_size: vec2<f32>,
}

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
}

struct InstanceInput {
    // Position of the quad in screen pixels (top-left corner)
    @location(0) position: vec2<f32>,
    // Size of the quad in pixels
    @location(1) size: vec2<f32>,
    // UV coordinates in atlas (top-left corner), zero for solid color
    @location(2) uv_offset: vec2<f32>,
    // UV size in atlas, zero for solid color
    @location(3) uv_size: vec2<f32>,
    // RGBA color (all outputs use premultiplied alpha blending)
    @location(4) color: vec4<f32>,
    // Flags: bit 0 = is_textured (use atlas alpha), bit 1 = is_colored_glyph
    @location(5) flags: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) flags: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var glyph_atlas: texture_2d<f32>;

@group(1) @binding(1)
var atlas_sampler: sampler;

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var output: VertexOutput;

    // Generate quad corners from vertex index (0-5 for two triangles)
    // Triangle 1: 0-1-2, Triangle 2: 3-4-5
    // Vertices: 0=TL, 1=TR, 2=BL, 3=BL, 4=TR, 5=BR
    var corner: vec2<f32>;
    let idx = vertex.vertex_index % 6u;
    if idx == 0u {
        corner = vec2<f32>(0.0, 0.0); // TL
    } else if idx == 1u {
        corner = vec2<f32>(1.0, 0.0); // TR
    } else if idx == 2u {
        corner = vec2<f32>(0.0, 1.0); // BL
    } else if idx == 3u {
        corner = vec2<f32>(0.0, 1.0); // BL
    } else if idx == 4u {
        corner = vec2<f32>(1.0, 0.0); // TR
    } else {
        corner = vec2<f32>(1.0, 1.0); // BR
    }

    // Calculate position in screen space
    let screen_pos = instance.position + corner * instance.size;

    // Apply projection
    output.clip_position = uniforms.projection * vec4<f32>(screen_pos, 0.0, 1.0);

    // Calculate texture coordinates
    output.tex_coord = instance.uv_offset + corner * instance.uv_size;

    // Pass through color and flags
    output.color = instance.color;
    output.flags = instance.flags;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let is_textured = (input.flags & 1u) != 0u;
    let is_colored_glyph = (input.flags & 2u) != 0u;

    if is_textured {
        if is_colored_glyph {
            // Color glyph (emoji): use texture color directly
            let tex_color = textureSample(glyph_atlas, atlas_sampler, input.tex_coord);
            return tex_color;
        } else {
            // Grayscale glyph: use texture alpha with text color
            let alpha = textureSample(glyph_atlas, atlas_sampler, input.tex_coord).a;
            return vec4<f32>(input.color.rgb * alpha, alpha * input.color.a);
        }
    } else {
        // Solid color quad (cell background)
        return input.color;
    }
}
