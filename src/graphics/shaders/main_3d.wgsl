struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) uv: vec2f,
    @location(1) tex_index: u32,
    @location(2) color: vec4f,
    @location(3) normal: vec3f,
    @location(4) radial_proper_velocity: f32,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) uv: vec2f,
    @location(2) tex_index: u32,
    @location(3) normal: vec3f,
}

struct InstanceInput {
    @location(4) model_matrix_0: vec4f,
    @location(5) model_matrix_1: vec4f,
    @location(6) model_matrix_2: vec4f,
    @location(7) model_matrix_3: vec4f,
    @location(8) velocity: vec3f,
    @location(9) color: vec4f,
}

struct CameraUniform {
    view_projection: mat4x4f,
    _padding: vec3u, // this is dumb
    aspect_ratio: f32,
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

fn rgb_to_hsv(c: vec3f) -> vec3f {
    let cmin = min(min(c.r, c.g), c.b);
    let cmax = max(max(c.r, c.g), c.b);
    let diff = cmax - cmin;
    var h = 0.0;
    var s = 0.0;

    if (cmax > 0.0) {
        s = diff / cmax;
    }

    if (diff > 0.0) {
        if (cmax == c.r) {
            h = (c.g - c.b) / diff;
            if (h < 0.0) {
                h += 6.0;
            }
        } else if (cmax == c.g) {
            h = 2.0 + (c.b - c.r) / diff;
        } else {
            h = 4.0 + (c.r - c.g) / diff;
        }
    }

    h /= 6.0;
    return vec3f(h, s, cmax);
}

fn hsv_to_rgb(c: vec3f) -> vec3f {
    let h = c.x * 6.0;
    let s = c.y;
    let v = c.z;

    let i = floor(h);
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    if (i == 0.0) {
        return vec3f(v, t, p);
    } else if (i == 1.0) {
        return vec3f(q, v, p);
    } else if (i == 2.0) {
        return vec3f(p, v, t);
    } else if (i == 3.0) {
        return vec3f(p, q, v);
    } else if (i == 4.0) {
        return vec3f(t, p, v);
    } else {
        return vec3f(v, p, q);
    }
}

@vertex
fn vert_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4f(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let rotation_matrix = mat3x3f(
        instance.model_matrix_0.xyz,
        instance.model_matrix_1.xyz,
        instance.model_matrix_2.xyz,
    );

    let origin_position = model_matrix * vec4f(0.0, 0.0, 0.0, 1.0);
    let actual_position = model_matrix * vec4f(model.position, 1.0);

    // terrell rotation (further-away vertices lag behind)
    let light_delay_offset = length(actual_position.xyz) - length(origin_position.xyz);
    let apparent_position = vec4f(actual_position.xyz - instance.velocity * light_delay_offset, 1.0);

    let radial_velocity = dot(apparent_position.xyz, -instance.velocity) / length(apparent_position.xyz);

    var out: VertexOutput;

    out.clip_position = camera.view_projection * apparent_position;
    out.uv = model.uv;
    out.tex_index = model.tex_index;
    out.color = instance.color;
    out.normal = normalize(rotation_matrix * model.normal);
    out.radial_proper_velocity = radial_velocity / sqrt(1.0 - length(radial_velocity));

    return out;
}

@group(0) @binding(0)
var texture_diffuse: texture_2d_array<f32>;
@group(0) @binding(1)
var sampler_diffuse: sampler;

@fragment
fn frag_main(in: VertexOutput) -> @location(0) vec4f {
    var directions = array<vec3f, 6>(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), vec3f(-1.0, 0.0, 0.0), vec3f(0.0, -1.0, 0.0), vec3f(0.0, 0.0, -1.0));
    var brightnesses = array<f32, 6>(0.8, 1.0, 0.7, 0.6, 0.4, 0.75);

    var color_multiplier = 0.0;
    for (var i = 0; i < 6; i++) {
        color_multiplier += (max(dot(normalize(in.normal), directions[i]) * brightnesses[i], 0.0));
    }

    let pixel_color = textureSample(texture_diffuse, sampler_diffuse, in.uv, in.tex_index) * in.color * vec4f(vec3f(color_multiplier), 1.0);

    // red/blue shift
    var red = rgb_to_hsv(vec3f(1.0, 0.0, 0.0));
    var green = rgb_to_hsv(vec3f(0.0, 1.0, 0.0));
    var blue = rgb_to_hsv(vec3f(0.0, 0.0, 1.0));

    let shift = in.radial_proper_velocity;

    red.x = clamp(red.x + shift, 0.0, 1.0);
    green.x = clamp(green.x + shift, 0.0, 1.0);
    blue.x += clamp(blue.x + shift, 0.0, 1.0);

    var shifted_color = hsv_to_rgb(red) * pixel_color.x + hsv_to_rgb(green) * pixel_color.y + hsv_to_rgb(blue) * pixel_color.z;
    shifted_color /= max(max(max(shifted_color.x, shifted_color.y), shifted_color.z), 1.0);
    return vec4f(shifted_color, pixel_color.w);
}