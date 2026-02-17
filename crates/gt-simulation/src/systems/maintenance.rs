use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    for health in world.healths.values_mut() {
        health.degrade(0.001);
    }
}
