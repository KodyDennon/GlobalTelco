use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    let mut costs = Vec::new();

    for (&entity, financial) in &world.financials {
        if financial.cost_per_tick > 0 {
            costs.push((entity, financial.cost_per_tick));
        }
    }

    for (entity, amount) in costs {
        if let Some(financial) = world.financials.get_mut(&entity) {
            financial.cash -= amount;
        }
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::CostIncurred { corporation: entity, amount },
        );
    }
}
