// Coverage compute shader — calculates per-cell coverage from node positions and radii.
// Dispatched as workgroups of 64 threads, one thread per grid cell.

struct Node {
    lon: f32,
    lat: f32,
    coverage_radius: f32, // in Mercator units
    bandwidth: f32,
    owner: u32,
    active: u32,
    _pad0: u32,
    _pad1: u32,
};

struct Cell {
    lon: f32,
    lat: f32,
};

struct Params {
    node_count: u32,
    cell_count: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read> nodes: array<Node>;
@group(0) @binding(1) var<storage, read> cells: array<Cell>;
@group(0) @binding(2) var<storage, read_write> coverage: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

// Approximate Haversine for small distances (Mercator-space Euclidean is fine for coverage)
fn distance_mercator(a_lon: f32, a_lat: f32, b_lon: f32, b_lat: f32) -> f32 {
    let dx = a_lon - b_lon;
    let dy = a_lat - b_lat;
    return sqrt(dx * dx + dy * dy);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cell_idx = id.x;
    if (cell_idx >= params.cell_count) {
        return;
    }

    let cell = cells[cell_idx];
    var total_coverage: f32 = 0.0;

    for (var i: u32 = 0u; i < params.node_count; i++) {
        let node = nodes[i];
        if (node.active == 0u) {
            continue;
        }

        let dist = distance_mercator(cell.lon, cell.lat, node.lon, node.lat);
        if (dist < node.coverage_radius) {
            // Linear falloff from center to edge of coverage radius
            let factor = 1.0 - dist / node.coverage_radius;
            total_coverage += node.bandwidth * factor;
        }
    }

    coverage[cell_idx] = total_coverage;
}
