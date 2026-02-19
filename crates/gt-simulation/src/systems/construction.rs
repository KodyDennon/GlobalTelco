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
            // Revenue is calculated by the revenue system each tick — no bonus here
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::ConstructionCompleted { entity, tick },
        );
    }
}
