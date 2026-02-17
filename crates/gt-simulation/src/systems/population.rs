use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    for pop in world.populations.values_mut() {
        let growth = (pop.count as f64 * pop.growth_rate) as u64;
        pop.count = pop.count.saturating_add(growth);
    }
}
