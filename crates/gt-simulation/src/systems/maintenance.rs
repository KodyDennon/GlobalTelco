use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    // Collect owner info for health-based maintenance decisions
    let mut nodes_info: Vec<(u64, u64, i64)> = world
        .infra_nodes
        .iter()
        .map(|(&id, node)| (id, node.owner, node.maintenance_cost))
        .collect();
    nodes_info.sort_unstable_by_key(|t| t.0);

    for (node_id, owner_id, maintenance_cost) in nodes_info {
        let has_budget = world
            .financials
            .get(&owner_id)
            .map(|f| f.cash > 0)
            .unwrap_or(false);

        if let Some(health) = world.healths.get_mut(&node_id) {
            if has_budget {
                // With budget: slow degradation, repair if damaged
                health.degrade(0.0005);
                if health.condition < 0.8 {
                    // Repair: costs extra but restores condition
                    health.condition = (health.condition + 0.002).min(1.0);
                }
            } else {
                // No budget: faster degradation
                health.degrade(0.003);
            }

            // Severely damaged infrastructure reduces throughput
            if health.condition < 0.5 {
                if let Some(node) = world.infra_nodes.get(&node_id) {
                    let max_tp = node.max_throughput;
                    if let Some(cap) = world.capacities.get_mut(&node_id) {
                        cap.max_throughput = max_tp * health.condition;
                    }
                }
            }

            // Track maintenance cost on the health component
            health.maintenance_cost_per_tick = maintenance_cost;
        }
    }

    // Degrade non-infrastructure health components normally
    let mut non_infra_health_ids: Vec<u64> = world
        .healths
        .keys()
        .filter(|id| !world.infra_nodes.contains_key(id))
        .copied()
        .collect();
    non_infra_health_ids.sort_unstable();

    for id in non_infra_health_ids {
        if let Some(health) = world.healths.get_mut(&id) {
            health.degrade(0.001);
        }
    }
}
