use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    let mut earnings = Vec::new();

    for (&entity, financial) in &world.financials {
        if financial.revenue_per_tick > 0 {
            earnings.push((entity, financial.revenue_per_tick));
        }
    }

    for (entity, amount) in earnings {
        if let Some(financial) = world.financials.get_mut(&entity) {
            financial.cash += amount;
        }
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::RevenueEarned { corporation: entity, amount },
        );
    }
}
