use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    if !world.network.has_dirty_nodes() {
        return;
    }

    // Build edge weight function from current edge data
    // Weight = latency * (1 / bandwidth) — lower is better routes
    let edge_weights: std::collections::HashMap<(u64, u64), f64> = world
        .infra_edges
        .values()
        .map(|edge| {
            let weight = edge.latency_ms * (1.0 / edge.bandwidth.max(1.0));
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
        edge_weights.get(&key).copied().unwrap_or(1.0)
    };

    world.network.recompute_dirty(&weight_fn);
}
