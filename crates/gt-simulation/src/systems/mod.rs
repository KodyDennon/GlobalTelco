pub mod achievement;
pub mod ai;
pub mod alliance;
pub mod auction;
pub mod construction;
pub mod contract;
pub mod cost;
pub mod coverage;
pub mod covert_ops;
pub mod demand;
pub mod disaster;
pub mod finance;
pub mod ftth;
pub mod grants;
pub mod legal;
pub mod lobbying;
pub mod maintenance;
pub mod market;
pub mod patent;
pub mod population;
pub mod regulation;
pub mod research;
pub mod revenue;
pub mod routing;
pub mod spectrum;
pub mod stock_market;
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
    spectrum::run(world);       // Carrier aggregation + interference penalties (after utilization resets capacities)
    ftth::run(world);           // Validate FTTH chains and mark active NAPs (after coverage, before revenue)
    revenue::run(world);
    cost::run(world);
    finance::run(world);
    contract::run(world);
    ai::run(world);
    disaster::run(world);
    regulation::run(world);
    research::run(world);
    patent::run(world);         // Royalty collection + lease expiration (Phase 5.3, after research)
    market::run(world);
    auction::run(world);
    covert_ops::run(world);
    lobbying::run(world);
    alliance::run(world);       // Trust scoring + auto-dissolution (Phase 5.1)
    legal::run(world);          // Lawsuit resolution + financial outcomes (Phase 5.2)
    grants::run(world);         // Grant generation + progress tracking + payouts (Phase 5.4)
    achievement::run(world);
    stock_market::run(world);   // Stock market price updates + auto-IPO (Phase 6.1)
    // Phase 8: Resolve spectrum auctions and expire licenses
    world.resolve_spectrum_auctions();
}
