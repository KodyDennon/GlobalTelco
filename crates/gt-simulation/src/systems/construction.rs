use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    let mut completed: Vec<u64> = world
        .constructions
        .iter()
        .filter(|(_, c)| c.is_complete(tick))
        .map(|(&id, _)| id)
        .collect();
    completed.sort_unstable();

    for entity in completed {
        world.constructions.remove(&entity);

        // When construction completes, activate the infrastructure
        if let Some(node) = world.infra_nodes.get(&entity) {
            let throughput = node.max_throughput;
            // Set capacity to full now that construction is done
            if let Some(cap) = world.capacities.get_mut(&entity) {
                cap.max_throughput = throughput;
            }
            // Add to network
            world.network.add_node(entity);

            // Generate revenue based on local demand
            let cell_idx = node.cell_index;
            let owner = node.owner;
            if let Some(region_id) = world.cell_to_region.get(&cell_idx).copied() {
                if let Some(demand) = world.demands.get(&region_id) {
                    let revenue = (demand.base_demand * 0.01 * throughput / 1000.0) as i64;
                    if let Some(fin) = world.financials.get_mut(&owner) {
                        fin.revenue_per_tick += revenue.max(1);
                    }
                }
            }
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::ConstructionCompleted { entity, tick },
        );
    }
}
