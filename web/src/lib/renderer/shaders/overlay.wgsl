// Overlay compute + render shaders — coverage, demand, utilization heatmaps.

struct Uniforms {
    matrix: mat4x4<f32>,
    viewport: vec2<f32>,
    cell_size: f32,  // Mercator units per cell
    _pad: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

// ── Overlay quad rendering ───────────────────────────────────────────────────
// Each cell is a colored quad. Instance data: position + value.

struct CellInstance {
    @location(0) mercator_pos: vec2<f32>,  // center of cell in Mercator
    @location(1) value: f32,               // 0..1 normalized overlay value
    @location(2) _pad: f32,
};

struct CellVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) value: f32,
};

const CELL_QUAD = array<vec2<f32>, 6>(
    vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(0.5, 0.5),
    vec2(-0.5, -0.5), vec2(0.5, 0.5),  vec2(-0.5, 0.5),
);

@vertex
fn vs_overlay(
    @builtin(vertex_index) vid: u32,
    inst: CellInstance,
) -> CellVertexOut {
    var out: CellVertexOut;

    let offset = CELL_QUAD[vid] * u.cell_size;
    let world_pos = inst.mercator_pos + offset;
    out.position = u.matrix * vec4(world_pos, 0.0, 1.0);
    out.value = inst.value;
    return out;
}

// Color ramp: blue (low) → green → yellow → red (high)
fn heatmap_color(t: f32) -> vec3<f32> {
    if (t < 0.25) {
        return mix(vec3(0.1, 0.1, 0.4), vec3(0.0, 0.5, 0.8), t * 4.0);
    } else if (t < 0.5) {
        return mix(vec3(0.0, 0.5, 0.8), vec3(0.1, 0.8, 0.2), (t - 0.25) * 4.0);
    } else if (t < 0.75) {
        return mix(vec3(0.1, 0.8, 0.2), vec3(0.95, 0.8, 0.0), (t - 0.5) * 4.0);
    } else {
        return mix(vec3(0.95, 0.8, 0.0), vec3(0.9, 0.2, 0.1), (t - 0.75) * 4.0);
    }
}

@fragment
fn fs_overlay(in: CellVertexOut) -> @location(0) vec4<f32> {
    if (in.value <= 0.0) {
        discard;
    }
    let color = heatmap_color(clamp(in.value, 0.0, 1.0));
    return vec4(color, 0.4);
}
