use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    let mut completed = Vec::new();

    for (&entity, construction) in &world.constructions {
        if construction.is_complete(tick) {
            completed.push(entity);
        }
    }

    for entity in completed {
        world.constructions.remove(&entity);
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::ConstructionCompleted { entity, tick },
        );
    }
}
