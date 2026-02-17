use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    for capacity in world.capacities.values_mut() {
        // Utilization adjusts toward demand-driven load
        capacity.current_load *= 0.95;
    }
}
