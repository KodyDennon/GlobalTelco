use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    if !world.network.has_dirty_nodes() {
        return;
    }

    // Build edge weight function: weight = latency * (1 / bandwidth) — lower is better.
    // Also factor in edge health: damaged edges have much higher weight.
    let edge_weights: std::collections::HashMap<(u64, u64), f64> = world
        .infra_edges
        .values()
        .map(|edge| {
            let effective_bw = edge.effective_bandwidth();
            let weight = if effective_bw <= 0.0 {
                f64::MAX // Edge is effectively destroyed
            } else {
                edge.latency_ms * (1.0 / effective_bw)
            };
            let key = if edge.source < edge.target {
                (edge.source, edge.target)
            } else {
                (edge.target, edge.source)
            };
            (key, weight)
        })
        .collect();

    let weight_fn = |a: u64, b: u64| -> f64 {
        let key = if a < b { (a, b) } else { (b, a) };
        edge_weights.get(&key).copied().unwrap_or(f64::MAX)
    };

    world.network.recompute_dirty(&weight_fn);
}
