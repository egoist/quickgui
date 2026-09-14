struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @location(0) pos: vec2<i32>,
    @location(1) dim: u32,
    @location(2) uv: u32,
    @location(3) color: u32,
    @location(4) content_type_with_srgb: u32,
    @location(5) depth: f32,
    @location(6) opacity: f32,
    @location(7) mask_bounds: vec4<f32>,
    @location(8) mask_radii: vec4<f32>,
}

struct VertexOutput {
    @invariant @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) content_type: u32,
    @location(3) @interpolate(flat) opacity: f32,
    @location(4) @interpolate(flat) color_mode: u32,
    @location(5) pixel_position: vec2<f32>,
    @location(6) @interpolate(flat) mask_bounds: vec4<f32>,
    @location(7) @interpolate(flat) mask_radii: vec4<f32>,
};

struct Params {
    screen_resolution: vec2<u32>,
    _pad: vec2<u32>,
};

@group(0) @binding(0)
var color_atlas_texture: texture_2d<f32>;

@group(0) @binding(1)
var mask_atlas_texture: texture_2d<f32>;

@group(0) @binding(2)
var atlas_sampler: sampler;

@group(1) @binding(0)
var<uniform> params: Params;

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    } else {
        return pow((c + 0.055) / 1.055, 2.4);
    }
}

@vertex
fn vs_main(in_vert: VertexInput) -> VertexOutput {
    var pos = in_vert.pos;
    let width = in_vert.dim & 0xffffu;
    let height = (in_vert.dim & 0xffff0000u) >> 16u;
    let color = in_vert.color;
    var uv = vec2<u32>(in_vert.uv & 0xffffu, (in_vert.uv & 0xffff0000u) >> 16u);
    let v = in_vert.vertex_idx;

    let corner_position = vec2<u32>(
        in_vert.vertex_idx & 1u,
        (in_vert.vertex_idx >> 1u) & 1u,
    );

    let corner_offset = vec2<u32>(width, height) * corner_position;

    uv = uv + corner_offset;
    pos = pos + vec2<i32>(corner_offset);

    var vert_output: VertexOutput;
    vert_output.pixel_position = vec2<f32>(pos);
    vert_output.mask_bounds = in_vert.mask_bounds;
    vert_output.mask_radii = in_vert.mask_radii;

    vert_output.position = vec4<f32>(
        2.0 * vec2<f32>(pos) / vec2<f32>(params.screen_resolution) - 1.0,
        in_vert.depth,
        1.0,
    );

    vert_output.position.y *= -1.0;

    let content_type = in_vert.content_type_with_srgb & 0xffffu;
    let srgb = (in_vert.content_type_with_srgb & 0xffff0000u) >> 16u;
    vert_output.color_mode = srgb;

    switch srgb {
        case 0u, 2u: {
            vert_output.color = vec4<f32>(
                f32((color & 0x00ff0000u) >> 16u) / 255.0,
                f32((color & 0x0000ff00u) >> 8u) / 255.0,
                f32(color & 0x000000ffu) / 255.0,
                f32((color & 0xff000000u) >> 24u) / 255.0,
            );
        }
        case 1u: {
            vert_output.color = vec4<f32>(
                srgb_to_linear(f32((color & 0x00ff0000u) >> 16u) / 255.0),
                srgb_to_linear(f32((color & 0x0000ff00u) >> 8u) / 255.0),
                srgb_to_linear(f32(color & 0x000000ffu) / 255.0),
                f32((color & 0xff000000u) >> 24u) / 255.0,
            );
        }
        default: {}
    }

    var dim: vec2<u32> = vec2(0u);
    switch content_type {
        case 0u: {
            dim = textureDimensions(color_atlas_texture);
            break;
        }
        case 1u: {
            dim = textureDimensions(mask_atlas_texture);
            break;
        }
        default: {}
    }

    vert_output.content_type = content_type;
    vert_output.opacity = clamp(in_vert.opacity, 0.0, 1.0);

    vert_output.uv = vec2<f32>(uv) / vec2<f32>(dim);

    return vert_output;
}

@fragment
fn fs_main(in_frag: VertexOutput) -> @location(0) vec4<f32> {
    var opacity = in_frag.opacity;
    if in_frag.mask_bounds.z > 0.0 && in_frag.mask_bounds.w > 0.0 {
        let p = in_frag.pixel_position - in_frag.mask_bounds.xy;
        let half_size = in_frag.mask_bounds.zw * 0.5;
        let top_radius = select(in_frag.mask_radii.x, in_frag.mask_radii.y, p.x > half_size.x);
        let bottom_radius = select(in_frag.mask_radii.w, in_frag.mask_radii.z, p.x > half_size.x);
        let radius = min(select(top_radius, bottom_radius, p.y > half_size.y), min(half_size.x, half_size.y));
        let q = abs(p - half_size) - half_size + vec2<f32>(radius);
        let distance = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
        opacity *= clamp(0.5 - distance / max(fwidth(distance), 0.0001), 0.0, 1.0);
    }
    switch in_frag.content_type {
        case 0u: {
            let sampled = textureSampleLevel(color_atlas_texture, atlas_sampler, in_frag.uv, 0.0);
            return vec4<f32>(sampled.rgb, sampled.a * opacity);
        }
        case 1u: {
            var coverage = textureSampleLevel(mask_atlas_texture, atlas_sampler, in_frag.uv, 0.0).x;
            if in_frag.color_mode == 2u {
                coverage = platform_coverage(coverage, in_frag.color.rgb);
            }
            return vec4<f32>(
                in_frag.color.rgb,
                in_frag.color.a
                    * coverage
                    * opacity,
            );
        }
        default: {
            return vec4<f32>(0.0);
        }
    }
}

// DirectWrite's correction polynomial, as used by GPUI's Windows/Linux renderer.
// Formula and gamma 1.8 coefficients adapted from Microsoft Terminal's dwrite.hlsl/dwrite.cpp.
// Copyright (c) Microsoft Corporation. Licensed under the MIT license.
fn platform_coverage(alpha: f32, color: vec3<f32>) -> f32 {
    let brightness = dot(color, vec3<f32>(0.30, 0.59, 0.11));
    let contrast = clamp(4.0 * (0.75 - brightness), 0.0, 1.0);
    let a = alpha * (contrast + 1.0) / (alpha * contrast + 1.0);
    let g = vec4<f32>(0.1469, -0.8911, 1.4644, -0.3234)
        * vec4<f32>(65536.0 / 65025.0, 256.0 / 255.0, 65536.0 / 65025.0, 256.0 / 255.0);
    return clamp(a + a * (1.0 - a) * ((g.x * brightness + g.y) * a + g.z * brightness + g.w), 0.0, 1.0);
}
