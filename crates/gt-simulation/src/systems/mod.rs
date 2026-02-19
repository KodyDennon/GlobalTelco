pub mod achievement;
pub mod ai;
pub mod auction;
pub mod construction;
pub mod contract;
pub mod cost;
pub mod coverage;
pub mod covert_ops;
pub mod demand;
pub mod disaster;
pub mod finance;
pub mod lobbying;
pub mod maintenance;
pub mod market;
pub mod population;
pub mod regulation;
pub mod research;
pub mod revenue;
pub mod routing;
pub mod utilization;

use crate::world::GameWorld;

pub fn run_all_systems(world: &mut GameWorld) {
    construction::run(world);
    maintenance::run(world);
    population::run(world);
    coverage::run(world);       // Calculate per-cell coverage from infrastructure
    demand::run(world);         // Uses coverage data for satisfaction
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
    auction::run(world);
    covert_ops::run(world);
    lobbying::run(world);
    achievement::run(world);
}
