struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tex_index: u32,
    @location(3) color: vec4<f32>,
}

@vertex
fn vert_main(
    model: VertexInput,
) -> VertexOutput {
    let x = model.position.x;
    let y = model.position.y;

    var out: VertexOutput;

    out.clip_position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = model.uv;
    out.color = model.color;

    return out;
}

@group(0) @binding(0)
var texture_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var sampler_diffuse: sampler;
@group(1) @binding(0)
var texture_mask: texture_2d<f32>;
@group(1) @binding(1)
var sampler_mask: sampler;
@fragment
fn frag_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(texture_diffuse, sampler_diffuse, in.uv);
    let mask_color = textureSample(texture_mask, sampler_mask, in.uv);
    if (mask_color.w > 0.5) {
        return vec4<f32>(vec3<f32>(1.0, 1.0, 1.0) - color.xyz, color.w);
    } else {
        return color;
    }
    
}