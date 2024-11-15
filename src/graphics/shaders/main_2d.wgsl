struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) tex_index: u32,
    @location(2) color: vec4f,
}

struct VertexInput {
    @location(0) position: vec2f,
    @location(1) uv: vec2f,
    @location(2) tex_index: u32,
    @location(3) color: vec4f,
}

@vertex
fn vert_main(
    model: VertexInput,
) -> VertexOutput {
    let x = model.position.x;
    let y = model.position.y;

    var out: VertexOutput;

    out.clip_position = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = model.uv;
    out.tex_index = model.tex_index;
    out.color = model.color;

    return out;
}

@group(0) @binding(0)
var texture_diffuse: texture_2d_array<f32>;
@group(0) @binding(1)
var sampler_diffuse: sampler;

@fragment
fn frag_main(in: VertexOutput) -> @location(0) vec4f {
    return textureSample(texture_diffuse, sampler_diffuse, in.uv, in.tex_index) * in.color;
}