struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct VertexInput {
    @location(0) index: u32,
}

struct Instance {
    @location(1) start: vec3<f32>,
    @location(2) end: vec3<f32>,
    @location(3) color: vec4<f32>,
}

struct CameraUniform {
    view_projection: mat4x4<f32>,
    _padding: vec3<u32>,
    aspect_ratio: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

const LINE_THICKNESS: f32 = 0.005;

@vertex
fn vert_main(
    model: VertexInput,
    instance: Instance,
) -> VertexOutput {
    var out: VertexOutput;

    var start_h = camera.view_projection * vec4<f32>(instance.start, 1.0) - vec4<f32>(0.0, 0.0, 0.00005, 0.0);
    var end_h = camera.view_projection * vec4<f32>(instance.end, 1.0) - vec4<f32>(0.0, 0.0, 0.00005, 0.0);

    let start_clipped = start_h.w <= 0.0;
    let end_clipped = end_h.w <= 0.0;

    if start_clipped && end_clipped {
        start_h = vec4<f32>(0.0, 0.0, -2.0, 1.0);
        end_h = vec4<f32>(0.0, 0.0, -3.0, 1.0);
    } else if start_clipped {
        let t = -start_h.w / (end_h.w - start_h.w);
        start_h += (end_h - start_h) * t;
        start_h.w = 0.0001;
    } else if end_clipped {
        let t = -end_h.w / (start_h.w - end_h.w);
        end_h += (start_h - end_h) * t;
        end_h.w = 0.0001;
    }

    let start_ndc = start_h.xyz / start_h.w;
    let end_ndc = end_h.xyz / end_h.w;

    let start = vec3<f32>(start_ndc.x * camera.aspect_ratio, start_ndc.yz);
    let end = vec3<f32>(end_ndc.x * camera.aspect_ratio, end_ndc.yz);

    let normal = normalize(end.xy - start.xy);
    let left_vector = vec3<f32>(-normal.y, normal.x, 0.0);
    let left_half_thickness = left_vector * LINE_THICKNESS / 2.0;
    
    let end_left = end + left_half_thickness;
    let end_right = end - left_half_thickness;
    let start_left = start + left_half_thickness;
    let start_right = start - left_half_thickness;

    var position: vec3<f32>;
    switch model.index {
        case 0u, default: {
            position = end_left;
        }
        case 1u: {
            position = start_left;
        }
        case 2u: {
            position = start_right;
        }
        case 3u: {
            position = end_right;
        } 
    }

    out.clip_position = vec4<f32>(position.x / camera.aspect_ratio, position.y, position.z, 1.0);
    out.color = instance.color;

    return out;
}

@fragment
fn frag_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}