struct TimeUniform {
    time: f32,
    delta: f32,
    frame: u32,
    _padding: u32,
}

struct Params {
    square_size: f32,
    circle_radius: f32,
    edge_thickness: f32,
    animation_speed: f32,
    background_color: vec3<f32>,
    edge_color_intensity: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
    _pad4: f32,
}

@group(0) @binding(0) var<uniform> time_data: TimeUniform;
@group(1) @binding(0) var output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var<uniform> params: Params;

const PI: f32 = 3.14159265358979323846;

fn luminance_modulation(t: f32, phase: f32) -> f32 {
    return 0.5 + 0.3 * sin(t + phase);
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let resolution = textureDimensions(output);
    let coord = vec2<i32>(global_id.xy);

    if (coord.x >= i32(resolution.x) || coord.y >= i32(resolution.y)) {
        return;
    }

    let frag_coord = vec2<f32>(coord) + vec2<f32>(0.5);
    let uv = (frag_coord - 0.5 * vec2<f32>(resolution)) / f32(resolution.y);
    var frag_color = vec4<f32>(params.background_color, 1.0);

    let t = time_data.time * params.animation_speed;
    let s = params.square_size;
    let r = params.circle_radius;
    let e = params.edge_thickness;
    let p = PI * 0.5;
    let centers = array<vec2<f32>, 4>(
        vec2<f32>(s, s),
        vec2<f32>(-s, s),
        vec2<f32>(-s, -s),
        vec2<f32>(s, -s)
    );

    for (var i: i32 = 0; i < 4; i = i + 1) {
        let local = uv - centers[i];

        if (length(local) < r) {
            let mx = sign(-centers[i].x);
            let my = sign(-centers[i].y);
            let mask = step(0.0, mx * local.x) * step(0.0, my * local.y);

            if (mask < 0.5) {
                let color = vec3<f32>(luminance_modulation(t, 0.0));
                frag_color = vec4<f32>(color, frag_color.a);
            } else {
                let h = step(abs(local.y), e);
                let v = step(abs(local.x), e);

                if (h > 0.5 && mx * local.x > 0.0) {
                    let hp = p * (1.0 - 2.0 * f32(i & 1));
                    let color = vec3<f32>(luminance_modulation(t, hp) * params.edge_color_intensity);
                    frag_color = vec4<f32>(color, frag_color.a);
                } else if (v > 0.5 && my * local.y > 0.0) {
                    let vp = p * (f32(i & 1) * 2.0 - 1.0);
                    let color = vec3<f32>(luminance_modulation(t, vp) * params.edge_color_intensity);
                    frag_color = vec4<f32>(color, frag_color.a);
                }
            }
        }
    }

    textureStore(output, coord, frag_color);
}
