struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct TransitionUniforms {
    progress: f32,
    mode: u32, // 0 = CrossDissolve, 1 = DipToColor
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> params: TransitionUniforms;
@group(0) @binding(1) var t_from: texture_2d<f32>;
@group(0) @binding(2) var s_from: sampler;
@group(0) @binding(3) var t_to: texture_2d<f32>;
@group(0) @binding(4) var s_to: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color_a = textureSample(t_from, s_from, in.uv);
    let color_b = textureSample(t_to, s_to, in.uv);
    let p = params.progress;

    if (params.mode == 0u) {
        // Cross dissolve
        return mix(color_a, color_b, p);
    } else {
        // Dip to black
        if (p < 0.5) {
            return mix(color_a, vec4<f32>(0.0, 0.0, 0.0, 1.0), p * 2.0);
        } else {
            return mix(vec4<f32>(0.0, 0.0, 0.0, 1.0), color_b, (p - 0.5) * 2.0);
        }
    }
}
