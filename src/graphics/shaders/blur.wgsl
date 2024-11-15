const TAU: f32 = 6.28318531;

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

@group(2) @binding(0)
var<uniform> aspect_ratio: f32;

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
    let radius = textureSample(texture_mask, sampler_mask, in.uv).w;

    let directions = 16;
    let radial_steps = 4;

    let angle_step = TAU / f32(directions);
    let radial_step = radius / f32(radial_steps);

    var color = vec4<f32>();
    var total = 0.0;
    for (var d = 0; d < directions; d++) {
        let angle = angle_step * f32(d);
        for (var r = 1; r <= radial_steps; r++) {
            let distance = radial_step * f32(r);
            color += textureSample(texture_diffuse, sampler_diffuse, in.uv + distance * vec2<f32>(cos(angle) / aspect_ratio, sin(angle)));
            total += 1.0;
        }
    }

    let final_color = (color / total);
    if final_color.w < 0.99 {
        return final_color;
    } else {
        return vec4<f32>(final_color.xyz, 1.0);
    }
}