// Infrastructure render shaders — nodes (instanced circles) and edges (line segments).

struct Uniforms {
    matrix: mat4x4<f32>,
    viewport: vec2<f32>,       // width, height in pixels
    time: f32,
    zoom: f32,
    selected_id: u32,
    hovered_id: u32,
    _pad: vec2<u32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

// ── Node Pipeline (instanced circles) ────────────────────────────────────────

struct NodeInstance {
    @location(0) mercator_pos: vec2<f32>,    // Mercator x, y
    @location(1) radius_color: vec4<f32>,    // radius (px), r, g, b
    @location(2) id_flags: vec2<u32>,        // entity ID, flags (bit 0 = under_construction, bit 1 = ghost)
};

struct NodeVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,              // -1..1 within circle quad
    @location(2) @interpolate(flat) flags: u32,
    @location(3) @interpolate(flat) entity_id: u32,
};

// Fullscreen quad vertices for instanced circles (2 triangles = 6 vertices)
const QUAD_POS = array<vec2<f32>, 6>(
    vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
    vec2(-1.0, -1.0), vec2(1.0, 1.0),  vec2(-1.0, 1.0),
);

@vertex
fn vs_node(
    @builtin(vertex_index) vid: u32,
    inst: NodeInstance,
) -> NodeVertexOut {
    var out: NodeVertexOut;

    let quad = QUAD_POS[vid];
    let radius_px = inst.radius_color.x;

    // Project Mercator position to clip space
    let clip = u.matrix * vec4(inst.mercator_pos, 0.0, 1.0);

    // Offset by quad position in pixel units, then convert to clip
    let pixel_offset = quad * radius_px;
    let ndc_offset = vec2(
        pixel_offset.x * 2.0 / u.viewport.x,
        pixel_offset.y * 2.0 / u.viewport.y,
    );

    out.position = vec4(clip.xy + ndc_offset * clip.w, clip.z, clip.w);
    out.color = inst.radius_color.yzw;
    out.uv = quad;
    out.flags = inst.id_flags.y;
    out.entity_id = inst.id_flags.x;
    return out;
}

@fragment
fn fs_node(in: NodeVertexOut) -> @location(0) vec4<f32> {
    // Circle SDF — discard pixels outside the circle
    let dist = length(in.uv);
    if (dist > 1.0) {
        discard;
    }

    var color = in.color;
    var alpha: f32 = 1.0;

    // Smooth antialiased edge
    alpha = 1.0 - smoothstep(0.85, 1.0, dist);

    // Under construction: pulsing effect
    if ((in.flags & 1u) != 0u) {
        alpha *= 0.5 + 0.3 * sin(u.time * 3.0);
    }

    // Ghost preview: translucent
    if ((in.flags & 2u) != 0u) {
        alpha *= 0.4;
    }

    // Selection ring
    if (in.entity_id == u.selected_id) {
        let ring = abs(dist - 0.75);
        if (ring < 0.08) {
            color = vec3(1.0, 1.0, 1.0);
            alpha = max(alpha, 0.9);
        }
    }

    // Hover highlight
    if (in.entity_id == u.hovered_id && in.entity_id != u.selected_id) {
        color = mix(color, vec3(1.0, 1.0, 1.0), 0.2);
    }

    return vec4(color, alpha);
}

// ── Edge Pipeline (line segments) ────────────────────────────────────────────

struct EdgeInstance {
    @location(0) src_mercator: vec2<f32>,    // source Mercator x, y
    @location(1) dst_mercator: vec2<f32>,    // destination Mercator x, y
    @location(2) width_color: vec4<f32>,     // width (px), r, g, b
    @location(3) id_util: vec2<f32>,         // entity ID (as f32), utilization 0..1
};

struct EdgeVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) utilization: f32,
    @location(2) @interpolate(flat) entity_id: u32,
    @location(3) line_coord: f32,            // 0..1 along edge for dashing
};

@vertex
fn vs_edge(
    @builtin(vertex_index) vid: u32,
    inst: EdgeInstance,
) -> EdgeVertexOut {
    var out: EdgeVertexOut;

    // Line segment as a thin quad: 6 vertices (2 triangles)
    // vid 0,1,2: first triangle; vid 3,4,5: second triangle
    let is_dst = (vid == 1u || vid == 2u || vid == 4u);
    let side: f32 = select(-1.0, 1.0, (vid == 2u || vid == 4u || vid == 5u));

    let src_clip = u.matrix * vec4(inst.src_mercator, 0.0, 1.0);
    let dst_clip = u.matrix * vec4(inst.dst_mercator, 0.0, 1.0);

    let pos_clip = select(src_clip, dst_clip, is_dst);
    let other_clip = select(dst_clip, src_clip, is_dst);

    // Direction perpendicular to the line in screen space
    let screen_pos = pos_clip.xy / pos_clip.w;
    let screen_other = other_clip.xy / other_clip.w;
    let dir = normalize(screen_pos - screen_other);
    let perp = vec2(-dir.y, dir.x);

    // Offset by half-width in pixels → NDC
    let half_width = inst.width_color.x * 0.5;
    let ndc_offset = perp * side * half_width * vec2(2.0 / u.viewport.x, 2.0 / u.viewport.y);

    out.position = vec4(pos_clip.xy + ndc_offset * pos_clip.w, pos_clip.z, pos_clip.w);
    out.color = inst.width_color.yzw;
    out.utilization = inst.id_util.y;
    // Edge ID was packed as uint32 bits into the f32 slot — bitcast to recover exact value.
    out.entity_id = bitcast<u32>(inst.id_util.x);
    out.line_coord = select(0.0, 1.0, is_dst);
    return out;
}

@fragment
fn fs_edge(in: EdgeVertexOut) -> @location(0) vec4<f32> {
    var color = in.color;
    var alpha: f32 = 0.8;

    // Color by utilization (green → yellow → red)
    if (in.utilization > 0.8) {
        color = mix(color, vec3(0.94, 0.27, 0.27), 0.5);
    } else if (in.utilization > 0.6) {
        color = mix(color, vec3(0.96, 0.62, 0.04), 0.3);
    }

    // Selection
    if (in.entity_id == u.selected_id) {
        color = vec3(1.0, 1.0, 1.0);
        alpha = 1.0;
    }

    // Hover
    if (in.entity_id == u.hovered_id && in.entity_id != u.selected_id) {
        color = mix(color, vec3(1.0, 1.0, 1.0), 0.25);
    }

    return vec4(color, alpha);
}
