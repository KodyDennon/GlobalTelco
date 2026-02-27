pub mod achievement;
pub mod ai;
pub mod alliance;
pub mod auction;
pub mod construction;
pub mod contract;
pub mod cost;
pub mod coverage;
pub mod covert_ops;
pub mod debris;
pub mod demand;
pub mod disaster;
pub mod finance;
pub mod ftth;
pub mod grants;
pub mod launch;
pub mod legal;
pub mod lobbying;
pub mod maintenance;
pub mod manufacturing;
pub mod market;
pub mod orbital;
pub mod patent;
pub mod population;
pub mod regulation;
pub mod research;
pub mod revenue;
pub mod routing;
pub mod satellite_network;
pub mod satellite_revenue;
pub mod servicing;
pub mod spectrum;
pub mod stock_market;
pub mod terminal_distribution;
pub mod utilization;
pub mod weather;

use crate::world::GameWorld;

pub fn run_all_systems(world: &mut GameWorld) {
    construction::run(world);
    orbital::run(world);                // Satellite orbital mechanics + position updates
    satellite_network::run(world);      // Dynamic ISL + downlink edge rebuild
    maintenance::run(world);
    population::run(world);
    coverage::run(world);               // Calculate per-cell coverage from infrastructure
    demand::run(world);                 // Uses coverage data for satisfaction
    routing::run(world);
    utilization::run(world);
    spectrum::run(world);               // Carrier aggregation + interference penalties
    ftth::run(world);                   // Validate FTTH chains and mark active NAPs
    manufacturing::run(world);          // Satellite + terminal factory production
    launch::run(world);                 // Process launch queues, send sats to orbit
    terminal_distribution::run(world);  // Distribute terminals to cities
    satellite_revenue::run(world);      // Satellite subscriber revenue
    revenue::run(world);
    cost::run(world);
    finance::run(world);
    contract::run(world);
    ai::run(world);
    weather::run(world);
    disaster::run(world);
    debris::run(world);                 // Orbital debris + Kessler cascade
    servicing::run(world);              // Satellite refuel/repair missions
    regulation::run(world);
    research::run(world);
    patent::run(world);
    market::run(world);
    auction::run(world);
    covert_ops::run(world);
    lobbying::run(world);
    alliance::run(world);
    legal::run(world);
    grants::run(world);
    achievement::run(world);
    stock_market::run(world);
    // Resolve spectrum auctions and expire licenses
    world.resolve_spectrum_auctions();
}
