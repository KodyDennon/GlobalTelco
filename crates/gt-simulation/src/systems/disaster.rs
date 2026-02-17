use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Disasters occur randomly based on region disaster risk
    // Check every 50 ticks to avoid excessive event spam
    if !tick.is_multiple_of(50) {
        return;
    }

    let mut region_data: Vec<(u64, f64, Vec<usize>)> = world
        .regions
        .iter()
        .map(|(&id, r)| (id, r.disaster_risk, r.cells.clone()))
        .collect();
    region_data.sort_unstable_by_key(|t| t.0);

    for (region_id, disaster_risk, cells) in region_data {
        let roll = world.deterministic_random();

        // Base 2% chance per check, scaled by disaster risk
        if roll < 0.02 * disaster_risk {
            let severity = world.deterministic_random() * 0.5 + 0.1; // 0.1 to 0.6

            // Damage infrastructure in this region
            let affected_nodes: Vec<u64> = world
                .infra_nodes
                .iter()
                .filter(|(_, node)| cells.contains(&node.cell_index))
                .map(|(&id, _)| id)
                .collect();

            for &node_id in &affected_nodes {
                if let Some(health) = world.healths.get_mut(&node_id) {
                    health.degrade(severity * 0.3);
                }
                // Reduce capacity during disaster
                if let Some(cap) = world.capacities.get_mut(&node_id) {
                    cap.max_throughput *= 1.0 - severity * 0.2;
                }
            }

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::DisasterStruck {
                    region: region_id,
                    severity,
                },
            );
        }
    }
}
