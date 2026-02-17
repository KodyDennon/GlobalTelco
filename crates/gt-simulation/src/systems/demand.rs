use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    for demand in world.demands.values_mut() {
        demand.current_demand = demand.base_demand;
    }
}
