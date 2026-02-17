pub mod construction;
pub mod maintenance;
pub mod population;
pub mod demand;
pub mod routing;
pub mod utilization;
pub mod revenue;
pub mod cost;
pub mod finance;
pub mod contract;
pub mod ai;
pub mod disaster;
pub mod regulation;
pub mod research;
pub mod market;

use crate::world::GameWorld;

pub fn run_all_systems(world: &mut GameWorld) {
    construction::run(world);
    maintenance::run(world);
    population::run(world);
    demand::run(world);
    routing::run(world);
    utilization::run(world);
    revenue::run(world);
    cost::run(world);
    finance::run(world);
    contract::run(world);
    ai::run(world);
    disaster::run(world);
    regulation::run(world);
    research::run(world);
    market::run(world);
}
