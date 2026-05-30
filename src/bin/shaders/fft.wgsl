// 2D FFT workflow with butterworth filter, Enes Altun 2025.
//  Some FFT operations adapted from: 
//  fadaaszhi, 2025: https://compute.toys/view/1187: Fast Fourier Transform
//  FabriceNeyret2, 2023 https://www.shadertoy.com/view/DtGfWV : Fourier Workflow 3 / phases info 
//  FabriceNeyret2, 2017  https://www.shadertoy.com/view/XtScWt: Fourier Workflow 2 / phases info 
//  SPIR-V-based Stockham + mixed-radix kernels   https://github.com/DTolm/VkFFT
//  Moreland, K., & Angel, E. (2003). The FFT on a GPU. SIGGRAPH/EUROGRAPHICS Conference On Graphics Hardware.

const RADIX = 4;

struct TimeUniform {
    time: f32,
    delta: f32,
    frame: u32,
    _padding: u32,
};
@group(0) @binding(0) var<uniform> time_data: TimeUniform;

struct FFTParams {
    filter_type: i32,     
    filter_strength: f32, 
    filter_direction: f32,
    filter_radius: f32,   
    show_freqs: i32,      
    resolution: u32,      
    is_bw: i32,
    _padding2: u32,
};
// Group 1: Primary Pass I/O & Parameters  
@group(1) @binding(0) var output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var<uniform> params: FFTParams;
@group(1) @binding(2) var input_texture: texture_2d<f32>;
@group(1) @binding(3) var input_sampler: sampler;

// Group 2: incoming audio FFT/spectrum from media, webcam, or Bespoke.
// Layout: 0..63 frequency bands, 64 BPM, 65 bass, 66 mid, 67 high, 68 total.
@group(2) @binding(0) var<storage, read> audio_spectrum: array<f32>;

// Storage buffer for FFT data
@group(3) @binding(0) var<storage, read_write> image_data: array<vec2f>;

const PI = 3.1415927;
const LOG2_N_MAX = 11;
const N_MAX = 2048;
const N_CHANNELS = 3u;
var<workgroup> X: array<vec2f, 2048>;

fn mul(x: vec2f, y: vec2f) -> vec2f {
    return vec2(x.x * y.x - x.y * y.y, x.x * y.y + x.y * y.x);
}

fn cis(x: f32) -> vec2f {
    return vec2(cos(x), sin(x));
}

fn index(channel: u32, y: u32, x: u32) -> u32 {
    let N = params.resolution;
    return channel * N * N + y * N + x;
}

fn reverse_bits(x: u32, bits: u32) -> u32 {
    var ret = 0u;
    var val = x;
    
    for(var i = 0u; i < bits; i++) {
        ret = (ret << 1u) | (val & 1u);
        val = val >> 1u;
    }
    
    return ret;
}

fn reverse_digits_base_4(x: u32, n: u32) -> u32 {
    var v = x;
    var y = 0u;
    
    for (var i = 0u; i < n; i++) {
        y = (y << 2u) | (v & 3u);
        v >>= 2u;
    }
    
    return y;
}

fn magnitude(z: vec2f) -> f32 {
    return sqrt(z.x * z.x + z.y * z.y);
}

fn phase(z: vec2f) -> f32 {
    return atan2(z.y, z.x);
}

fn fftshift(i: u32, N: u32) -> u32 {
    return (i + N / 2u) % N;
}

@compute @workgroup_size(16, 16, 1)
fn initialize_data(@builtin(global_invocation_id) id: vec3u) {
    let N = params.resolution;
    
    if (any(id.xy >= vec2(N))) {
        return;
    }
    
    let uv = (vec2f(id.xy) + 0.5) / f32(N);
    
    // Load color from input texture (uploaded by user) or black screen if no texture
    let input_size = vec2u(textureDimensions(input_texture));
    let sample_coord = vec2u(uv * vec2f(input_size));
    let clamped_coord = clamp(sample_coord, vec2u(0), input_size - vec2u(1));
    var color = textureLoad(input_texture, clamped_coord, 0).rgb;
    
    for (var i = 0u; i < N_CHANNELS; i++) {
        image_data[index(i, id.y, id.x)] = vec2(color[i], 0.0);
    }
}

// FFT on rows
@compute @workgroup_size(64, 1, 1)
fn fft_horizontal(@builtin(workgroup_id) workgroup_id: vec3u, @builtin(local_invocation_index) local_index: u32) {
    let LOG2_N = firstLeadingBit(params.resolution);
    let LOG4_N = LOG2_N / 2u;
    let N = params.resolution;
    
    let row = workgroup_id.x;
    if (row >= N) { return; }
    
    for (var ch = 0u; ch < N_CHANNELS; ch++) {
        // Load data with bit-reversal permutation
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            
            var k: u32;
            if (RADIX == 2) {
                k = reverse_bits(j, LOG2_N);
            } else {
                k = reverse_digits_base_4(j >> (LOG2_N & 1u), LOG4_N);
                k |= (j & (LOG2_N & 1u)) << (LOG2_N - 1u);
            }
            
            X[k] = image_data[index(ch, row, j)];
        }
        
        workgroupBarrier();
        
        // Radix-4 FFT passes
        for (var p = 0u; RADIX == 4 && p < LOG4_N; p++) {
            let s = 1u << (2u * p);
            
            for (var i = 0u; i < N / 64u / 4u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let t = -2.0 * PI / f32(s * 4u) * f32(k);
                let k0 = ((j >> (2u * p)) << (2u * p + 2u)) + k;
                let k1 = k0 + 1u * s;
                let k2 = k0 + 2u * s;
                let k3 = k0 + 3u * s;
                
                let x0 = X[k0];
                let x1 = mul(cis(t), X[k1]);
                let x2 = mul(cis(t * 2.0), X[k2]);
                let x3 = mul(cis(t * 3.0), X[k3]);
                
                X[k0] = x0 + x1 + x2 + x3;
                X[k1] = x0 - mul(vec2(0.0, 1.0), x1) - x2 + mul(vec2(0.0, 1.0), x3);
                X[k2] = x0 - x1 + x2 - x3;
                X[k3] = x0 + mul(vec2(0.0, 1.0), x1) - x2 - mul(vec2(0.0, 1.0), x3);
            }
            
            workgroupBarrier();
        }
        
        for (var p = select(0u, 2u * LOG4_N, RADIX == 4); p < LOG2_N; p++) {
            let s = 1u << p;
            
            for (var i = 0u; i < N / 64u / 2u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let k0 = ((j >> p) << (p + 1u)) + k;
                let k1 = k0 + s;
                
                let x0 = X[k0];
                let x1 = mul(cis(-2.0 * PI / f32(s * 2u) * f32(k)), X[k1]);
                
                X[k0] = x0 + x1;
                X[k1] = x0 - x1;
            }
            
            workgroupBarrier();
        }
        
        // Store results back to storage
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            image_data[index(ch, row, j)] = X[j];
        }
    }
}

// FFT on columns
@compute @workgroup_size(64, 1, 1)
fn fft_vertical(@builtin(workgroup_id) workgroup_id: vec3u, @builtin(local_invocation_index) local_index: u32) {
    let LOG2_N = firstLeadingBit(params.resolution);
    let LOG4_N = LOG2_N / 2u;
    let N = params.resolution;
    
    let col = workgroup_id.x;
    if (col >= N) { return; }
    
    for (var ch = 0u; ch < N_CHANNELS; ch++) {
        // Load data with bit-reversal permutation
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            
            var k: u32;
            if (RADIX == 2) {
                k = reverse_bits(j, LOG2_N);
            } else {
                k = reverse_digits_base_4(j >> (LOG2_N & 1u), LOG4_N);
                k |= (j & (LOG2_N & 1u)) << (LOG2_N - 1u);
            }
            
            X[k] = image_data[index(ch, j, col)];
        }
        
        workgroupBarrier();
        
        // Radix-4 FFT passes
        for (var p = 0u; RADIX == 4 && p < LOG4_N; p++) {
            let s = 1u << (2u * p);
            
            for (var i = 0u; i < N / 64u / 4u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let t = -2.0 * PI / f32(s * 4u) * f32(k);
                let k0 = ((j >> (2u * p)) << (2u * p + 2u)) + k;
                let k1 = k0 + 1u * s;
                let k2 = k0 + 2u * s;
                let k3 = k0 + 3u * s;
                
                let x0 = X[k0];
                let x1 = mul(cis(t), X[k1]);
                let x2 = mul(cis(t * 2.0), X[k2]);
                let x3 = mul(cis(t * 3.0), X[k3]);
                
                X[k0] = x0 + x1 + x2 + x3;
                X[k1] = x0 - mul(vec2(0.0, 1.0), x1) - x2 + mul(vec2(0.0, 1.0), x3);
                X[k2] = x0 - x1 + x2 - x3;
                X[k3] = x0 + mul(vec2(0.0, 1.0), x1) - x2 - mul(vec2(0.0, 1.0), x3);
            }
            
            workgroupBarrier();
        }
        
        for (var p = select(0u, 2u * LOG4_N, RADIX == 4); p < LOG2_N; p++) {
            let s = 1u << p;
            
            for (var i = 0u; i < N / 64u / 2u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let k0 = ((j >> p) << (p + 1u)) + k;
                let k1 = k0 + s;
                
                let x0 = X[k0];
                let x1 = mul(cis(-2.0 * PI / f32(s * 2u) * f32(k)), X[k1]);
                
                X[k0] = x0 + x1;
                X[k1] = x0 - x1;
            }
            
            workgroupBarrier();
        }
        
        // Store results back to storage
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            image_data[index(ch, j, col)] = X[j];
        }
    }
}

fn butterworth(f: f32, cutoff: f32, order: f32, highpass: bool) -> f32 {
    let ratio = f / cutoff;
    var result: f32;
    
    if (highpass) {
        result = 1.0 / (1.0 + pow(cutoff / max(f, 0.001), 2.0 * order));
    } else {
        result = 1.0 / (1.0 + pow(ratio, 2.0 * order));
    }
    
    return result;
}

// Frequency domain operations
@compute @workgroup_size(16, 16, 1)
fn modify_frequencies(@builtin(global_invocation_id) id: vec3u) {
    let N = params.resolution;
    
    if (any(id.xy >= vec2(N))) {
        return;
    }
    
    // Calculate shifted coordinates for centered frequency representation
    let shifted_x = (id.x + N / 2u) % N;
    let shifted_y = (id.y + N / 2u) % N;
    
    // Calculate frequency coordinates (0,0 is DC, center of the image)
    let freq_x = f32(shifted_x) - f32(N / 2u);
    let freq_y = f32(shifted_y) - f32(N / 2u);
    
    // Normalized frequency (distance from DC). range: [0, 1]
    let freq_coords = vec2f(freq_x, freq_y);
    let f = length(freq_coords) / f32(N / 2u);
    
    // No filtering when strength is zero
    if (params.filter_strength <= 0.001) {
        return;
    }

    var scale = 1.0;
    let strength = params.filter_strength;
    let order = 7.0;
    switch params.filter_type {
        // LP
        case 0: {
            // Butterworth low-pass: strength 0 → cutoff 1.0 (pass all), strength 1 → cutoff 0.01 (strong)
            let cutoff = mix(1.0, 0.01, strength);
            scale = butterworth(f, cutoff, order, false);
            break;
        }
        // HP
        case 1: {
            // Butterworth high-pass: strength 0 → cutoff 0.001 (pass all), strength 1 → cutoff 0.5 (strong)
            let cutoff = mix(0.001, 0.5, strength);
            scale = butterworth(f, cutoff, order, true);
            break;
        }
        // Band-pass filter
        case 2: {
            let center = params.filter_radius / 6.28;
            // Bandwidth: strength 0 → wide (0.3), strength 1 → narrow (0.02)
            let bandwidth = mix(0.3, 0.02, strength);
            scale = exp(-pow((f - center) / bandwidth, 2.0));
            break;
        }
        // Directional
        case 3: {
            let angle = atan2(freq_coords.y, freq_coords.x);
            let direction = params.filter_direction;
            // Angular width: strength 0 → wide (1.5), strength 1 → narrow (0.1)
            let angular_width = mix(1.5, 0.1, strength);
            scale = exp(-pow(sin(angle - direction) / angular_width, 2.0));
            break;
        }
        default: {
            scale = 1.0;
        }
    }
    
    // Apply the filter to each channel
    for (var i = 0u; i < N_CHANNELS; i++) {
        //(not shifted) position
        image_data[index(i, id.y, id.x)] *= scale;
    }
}

// inverse FFT on rows
@compute @workgroup_size(64, 1, 1)
fn ifft_horizontal(@builtin(workgroup_id) workgroup_id: vec3u, @builtin(local_invocation_index) local_index: u32) {
    let LOG2_N = firstLeadingBit(params.resolution);
    let LOG4_N = LOG2_N / 2u;
    let N = params.resolution;
    
    let row = workgroup_id.x;
    if (row >= N) { return; }
    
    for (var ch = 0u; ch < N_CHANNELS; ch++) {
        // Load data with bit-reversal permutation
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            
            var k: u32;
            if (RADIX == 2) {
                k = reverse_bits(j, LOG2_N);
            } else {
                k = reverse_digits_base_4(j >> (LOG2_N & 1u), LOG4_N);
                k |= (j & (LOG2_N & 1u)) << (LOG2_N - 1u);
            }
            
            X[k] = image_data[index(ch, row, j)];
        }
        
        workgroupBarrier();
        
        for (var p = 0u; RADIX == 4 && p < LOG4_N; p++) {
            let s = 1u << (2u * p);
            
            for (var i = 0u; i < N / 64u / 4u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let t = 2.0 * PI / f32(s * 4u) * f32(k);
                let k0 = ((j >> (2u * p)) << (2u * p + 2u)) + k;
                let k1 = k0 + 1u * s;
                let k2 = k0 + 2u * s;
                let k3 = k0 + 3u * s;
                
                let x0 = X[k0];
                let x1 = mul(cis(t), X[k1]);
                let x2 = mul(cis(t * 2.0), X[k2]);
                let x3 = mul(cis(t * 3.0), X[k3]);
                
                X[k0] = x0 + x1 + x2 + x3;
                X[k1] = x0 + mul(vec2(0.0, 1.0), x1) - x2 - mul(vec2(0.0, 1.0), x3);
                X[k2] = x0 - x1 + x2 - x3;
                X[k3] = x0 - mul(vec2(0.0, 1.0), x1) - x2 + mul(vec2(0.0, 1.0), x3);
            }
            
            workgroupBarrier();
        }
        
        for (var p = select(0u, 2u * LOG4_N, RADIX == 4); p < LOG2_N; p++) {
            let s = 1u << p;
            
            for (var i = 0u; i < N / 64u / 2u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let k0 = ((j >> p) << (p + 1u)) + k;
                let k1 = k0 + s;
                
                let x0 = X[k0];
                let x1 = mul(cis(2.0 * PI / f32(s * 2u) * f32(k)), X[k1]);
                
                X[k0] = x0 + x1;
                X[k1] = x0 - x1;
            }
            
            workgroupBarrier();
        }
        
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            image_data[index(ch, row, j)] = X[j] / f32(N);
        }
    }
}

// now on columns inverse... 
@compute @workgroup_size(64, 1, 1)
fn ifft_vertical(@builtin(workgroup_id) workgroup_id: vec3u, @builtin(local_invocation_index) local_index: u32) {
    let LOG2_N = firstLeadingBit(params.resolution);
    let LOG4_N = LOG2_N / 2u;
    let N = params.resolution;
    
    let col = workgroup_id.x;
    if (col >= N) { return; }
    
    for (var ch = 0u; ch < N_CHANNELS; ch++) {
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            
            var k: u32;
            if (RADIX == 2) {
                k = reverse_bits(j, LOG2_N);
            } else {
                k = reverse_digits_base_4(j >> (LOG2_N & 1u), LOG4_N);
                k |= (j & (LOG2_N & 1u)) << (LOG2_N - 1u);
            }
            
            X[k] = image_data[index(ch, j, col)];
        }
        
        workgroupBarrier();
        
        for (var p = 0u; RADIX == 4 && p < LOG4_N; p++) {
            let s = 1u << (2u * p);
            
            for (var i = 0u; i < N / 64u / 4u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let t = 2.0 * PI / f32(s * 4u) * f32(k);
                let k0 = ((j >> (2u * p)) << (2u * p + 2u)) + k;
                let k1 = k0 + 1u * s;
                let k2 = k0 + 2u * s;
                let k3 = k0 + 3u * s;
                
                let x0 = X[k0];
                let x1 = mul(cis(t), X[k1]);
                let x2 = mul(cis(t * 2.0), X[k2]);
                let x3 = mul(cis(t * 3.0), X[k3]);
                
                X[k0] = x0 + x1 + x2 + x3;
                X[k1] = x0 + mul(vec2(0.0, 1.0), x1) - x2 - mul(vec2(0.0, 1.0), x3);
                X[k2] = x0 - x1 + x2 - x3;
                X[k3] = x0 - mul(vec2(0.0, 1.0), x1) - x2 + mul(vec2(0.0, 1.0), x3);
            }
            
            workgroupBarrier();
        }
        
        for (var p = select(0u, 2u * LOG4_N, RADIX == 4); p < LOG2_N; p++) {
            let s = 1u << p;
            
            for (var i = 0u; i < N / 64u / 2u; i++) {
                let j = local_index + i * 64u;
                let k = j & (s - 1u);
                let k0 = ((j >> p) << (p + 1u)) + k;
                let k1 = k0 + s;
                
                let x0 = X[k0];
                let x1 = mul(cis(2.0 * PI / f32(s * 2u) * f32(k)), X[k1]);
                
                X[k0] = x0 + x1;
                X[k1] = x0 - x1;
            }
            
            workgroupBarrier();
        }
        
        for (var i = 0u; i < N / 64u; i++) {
            let j = local_index + i * 64u;
            image_data[index(ch, j, col)] = X[j] / f32(N);
        }
    }
}

fn audio_value(f: f32) -> f32 {
    let idx = clamp(f, 0.0, 0.999) * 64.0;
    let i = u32(idx);
    let frac_part = idx - f32(i);
    let a = audio_spectrum[i];
    let b = audio_spectrum[min(i + 1u, 63u)];
    return mix(a, b, frac_part);
}

fn audio_energy() -> f32 {
    return max(audio_spectrum[68], max(max(audio_spectrum[65], audio_spectrum[66]), audio_spectrum[67]));
}

fn render_audio_fft(tc: vec2f, t: f32) -> vec3f {
    let bass = audio_spectrum[65];
    let mid = audio_spectrum[66];
    let high = audio_spectrum[67];
    let total = audio_spectrum[68];

    var color = vec3f(0.004, 0.006, 0.012) + vec3f(0.015, 0.010, 0.020) * total;

    let band_width = 1.0 / 64.0;
    let band = clamp(u32(tc.x * 64.0), 0u, 63u);
    let f = (f32(band) + 0.5) / 64.0;
    let raw = audio_value(f);
    let shaped = min(1.0, pow(raw, mix(0.72, 0.48, f)) * mix(1.15, 1.85, f));
    let bar_top = 0.90 - shaped * 0.74;
    let bar_bottom = 0.90;
    let local_x = fract(tc.x / band_width);
    let in_bar = tc.y >= bar_top && tc.y <= bar_bottom && local_x > 0.16 && local_x < 0.84;

    let hue = vec3f(
        0.22 + 0.78 * smoothstep(0.00, 0.35, f),
        0.82 - 0.46 * smoothstep(0.38, 1.00, f),
        0.45 + 0.55 * smoothstep(0.42, 1.00, f)
    );
    if (in_bar) {
        let y_mix = (bar_bottom - tc.y) / max(bar_bottom - bar_top, 0.001);
        color = hue * (0.55 + y_mix * 0.95 + shaped * 0.65);
    }

    let line_y = 0.90 - shaped * 0.74;
    let line_dist = abs(tc.y - line_y);
    color += hue * exp(-line_dist * 125.0) * (0.20 + shaped * 0.90);

    let center = vec2f(0.5, 0.48);
    let p = tc - center;
    let radius = length(p);
    let angle = atan2(p.y, p.x) / (2.0 * PI) + 0.5;
    let radial_band = audio_value(angle);
    let ring = exp(-abs(radius - (0.18 + radial_band * 0.26)) * 32.0);
    color += vec3f(high, mid, bass) * ring * (0.22 + total * 0.75);

    let pulse = 0.5 + 0.5 * sin(t * 2.0 * PI * max(audio_spectrum[64] / 60.0, 0.25));
    color += vec3f(0.06, 0.04, 0.02) * bass * pulse * smoothstep(0.42, 0.0, radius);

    return clamp(color, vec3f(0.0), vec3f(1.0));
}

//render
@compute @workgroup_size(16, 16, 1)
fn main_image(@builtin(global_invocation_id) id: vec3u) {
    let dimensions = vec2u(textureDimensions(output));
    
    if (any(id.xy >= dimensions)) {
        return;
    }
    
    let N = params.resolution;
    let tc = vec2f(id.xy) / vec2f(dimensions);

    if (audio_energy() > 0.001) {
        var audio_color = render_audio_fft(tc, time_data.time);
        if (params.is_bw != 0) {
            let luminance = dot(audio_color, vec3(0.299, 0.587, 0.114));
            audio_color = vec3(luminance);
        }
        textureStore(output, id.xy, vec4(audio_color, 1.0));
        return;
    }
    
    // Calculate position in FFT image (centered)
    var p = vec2i(id.xy) - vec2i(dimensions) / 2 + vec2i(i32(N / 2u));
    
    // Check if position is within the FFT image bounds
    if (any((p < vec2i(0)) | (p >= vec2i(i32(N))))) {
        textureStore(output, id.xy, vec4(0.0, 0.0, 0.0, 1.0));
        return;
    }
    
    var color = vec3(0.0);
    
    if (params.show_freqs == 1) {
        // Frequency domain for better vis also log scaling for better dynamic rang
        
        let shift_x = (u32(p.x) + N / 2u) % N;
        let shift_y = (u32(p.y) + N / 2u) % N;
        
        for (var i = 0u; i < N_CHANNELS; i++) {
            let data = image_data[index(i, shift_y, shift_x)];
            

            let pixel_count = f32(N * N);
            let normalized_amp = length(data) / pixel_count;
            

            let exposure = 5000.0;
            color[i] = log(1.0 + normalized_amp * exposure) / log(exposure + 1.0);
        }
    } else {
        // Spatial domain (Filtered Image)
        for (var i = 0u; i < N_CHANNELS; i++) {
            let data = image_data[index(i, u32(p.y), u32(p.x))];
            color[i] = data.x;
        }
    }
    
    color = clamp(color, vec3(0.0), vec3(1.0));
    
    if (N_CHANNELS == 1u) {
        color = vec3(color.r);
    }
    
    if (params.is_bw != 0) {
        let luminance = dot(color, vec3(0.299, 0.587, 0.114));
        color = vec3(luminance);
    }
    textureStore(output, id.xy, vec4(color, 1.0));
}
