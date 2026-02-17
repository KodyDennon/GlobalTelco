use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    // Calculate utilization: distribute regional demand across infrastructure
    // Collect region demand data
    let mut region_demands: Vec<(u64, Vec<usize>, f64)> = world
        .regions
        .iter()
        .map(|(&id, region)| {
            let demand = world
                .demands
                .get(&id)
                .map(|d| d.current_demand)
                .unwrap_or(0.0);
            (id, region.cells.clone(), demand)
        })
        .collect();
    region_demands.sort_unstable_by_key(|t| t.0);

    // For each region, distribute demand across nodes in that region
    for (_region_id, cells, demand) in &region_demands {
        let region_nodes: Vec<u64> = world
            .infra_nodes
            .iter()
            .filter(|(id, node)| {
                cells.contains(&node.cell_index) && !world.constructions.contains_key(id)
            })
            .map(|(&id, _)| id)
            .collect();

        if region_nodes.is_empty() || *demand <= 0.0 {
            continue;
        }

        let total_capacity: f64 = region_nodes
            .iter()
            .filter_map(|id| world.capacities.get(id))
            .map(|c| c.max_throughput)
            .sum();

        if total_capacity <= 0.0 {
            continue;
        }

        // Distribute load proportionally to each node's capacity
        for &node_id in &region_nodes {
            if let Some(cap) = world.capacities.get_mut(&node_id) {
                let share = cap.max_throughput / total_capacity;
                let load = demand * share;
                // Smooth load changes: blend 80% new, 20% old
                cap.current_load = cap.current_load * 0.2 + load * 0.8;
                cap.current_load = cap.current_load.min(cap.max_throughput * 1.2);
                // Allow slight overload
            }
            // Also update infra node load
            if let Some(node) = world.infra_nodes.get_mut(&node_id) {
                if let Some(cap) = world.capacities.get(&node_id) {
                    node.current_load = cap.current_load;
                }
            }
        }
    }

    // Update edge utilization based on connected node loads
    let mut edge_updates: Vec<(u64, f64)> = world
        .infra_edges
        .iter()
        .map(|(&id, edge)| {
            let src_load = world
                .capacities
                .get(&edge.source)
                .map(|c| c.utilization())
                .unwrap_or(0.0);
            let dst_load = world
                .capacities
                .get(&edge.target)
                .map(|c| c.utilization())
                .unwrap_or(0.0);
            let avg_load = (src_load + dst_load) / 2.0;
            let edge_load = avg_load * edge.bandwidth;
            (id, edge_load)
        })
        .collect();
    edge_updates.sort_unstable_by_key(|t| t.0);

    for (edge_id, load) in edge_updates {
        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            edge.current_load = edge.current_load * 0.2 + load * 0.8;
        }
    }
}
