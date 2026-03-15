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

#[cfg(not(target_family = "wasm"))]
use indexmap::IndexMap;
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;

use crate::world::GameWorld;

// ── Dirty-bit constants (one bit per skippable system) ─────────────────────

pub const DIRTY_CONSTRUCTION: u64 = 1 << 0;
pub const DIRTY_DEBRIS: u64 = 1 << 3;
pub const DIRTY_SERVICING: u64 = 1 << 4;
pub const DIRTY_AUCTION: u64 = 1 << 5;
pub const DIRTY_LEGAL: u64 = 1 << 6;
pub const DIRTY_LOBBYING: u64 = 1 << 7;
pub const DIRTY_COVERT_OPS: u64 = 1 << 8;
pub const DIRTY_GRANTS: u64 = 1 << 9;
pub const DIRTY_ALLIANCE: u64 = 1 << 10;
pub const DIRTY_PATENT: u64 = 1 << 11;
pub const DIRTY_LAUNCH: u64 = 1 << 12;
pub const DIRTY_MANUFACTURING: u64 = 1 << 13;
pub const DIRTY_STOCK_MARKET: u64 = 1 << 14;
pub const DIRTY_SPECTRUM: u64 = 1 << 15;

/// Mark specific dirty flags on the world.
#[inline]
pub fn mark_dirty(world: &mut GameWorld, flags: u64) {
    world.dirty_flags |= flags;
}

/// Check if a specific dirty flag is set.
#[inline]
fn is_dirty(world: &GameWorld, flag: u64) -> bool {
    world.dirty_flags & flag != 0
}

/// Check if a system should run based on its dirty flag and whether
/// relevant data exists. Returns true if the system should run.
/// Grants and stock_market are always-run systems.
fn should_run_debris(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_DEBRIS) || !world.satellites.is_empty()
}

fn should_run_servicing(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_SERVICING) || !world.service_missions.is_empty()
}

fn should_run_auction(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_AUCTION) || !world.auctions.is_empty()
}

fn should_run_legal(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_LEGAL) || !world.lawsuits.is_empty()
}

fn should_run_lobbying(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_LOBBYING) || !world.lobbying_campaigns.is_empty()
}

fn should_run_covert_ops(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_COVERT_OPS) || !world.covert_ops.is_empty()
}

// Grants is an always-run system (generates new grants per region periodically).

fn should_run_alliance(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_ALLIANCE) || !world.alliances.is_empty()
}

fn should_run_patent(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_PATENT) || !world.patents.is_empty()
}

fn should_run_launch(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_LAUNCH)
        || !world.launch_pads.is_empty()
        || !world.pending_contract_launches.is_empty()
}

fn should_run_manufacturing(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_MANUFACTURING)
        || !world.satellite_factories.is_empty()
        || !world.terminal_factories.is_empty()
}

// Stock market is an always-run system (discovers IPO candidates among corps).

fn should_run_spectrum(world: &GameWorld) -> bool {
    is_dirty(world, DIRTY_SPECTRUM)
        || !world.spectrum_licenses.is_empty()
        || !world.spectrum_auctions.is_empty()
}

pub fn run_all_systems(world: &mut GameWorld) {
    // Always-run systems (core simulation loop)
    construction::run(world);
    orbital::run(world);
    satellite_network::run(world);
    maintenance::run(world);
    population::run(world);
    coverage::run(world);
    demand::run(world);
    routing::run(world);
    utilization::run(world);

    // Conditionally-run systems (skip if no relevant data)
    if should_run_spectrum(world) {
        spectrum::run(world);
    }
    ftth::run(world);
    if should_run_manufacturing(world) {
        manufacturing::run(world);
    }
    if should_run_launch(world) {
        launch::run(world);
    }
    terminal_distribution::run(world);
    satellite_revenue::run(world);
    revenue::run(world);
    cost::run(world);
    finance::run(world);
    contract::run(world);
    ai::run(world);
    if should_run_debris(world) {
        debris::run(world);
    }
    if should_run_servicing(world) {
        servicing::run(world);
    }
    regulation::run(world);
    research::run(world);
    if should_run_patent(world) {
        patent::run(world);
    }
    market::run(world);
    if should_run_auction(world) {
        auction::run(world);
    }
    if should_run_covert_ops(world) {
        covert_ops::run(world);
    }
    if should_run_lobbying(world) {
        lobbying::run(world);
    }
    if should_run_alliance(world) {
        alliance::run(world);
    }
    if should_run_legal(world) {
        legal::run(world);
    }
    // Grants always runs — generates new grants per region periodically.
    grants::run(world);
    achievement::run(world);
    // Stock market always runs — discovers IPO candidates among corps.
    stock_market::run(world);

    // Resolve spectrum auctions and expire licenses
    world.resolve_spectrum_auctions();

    // Reset dirty flags for next tick — commands will set them again
    world.dirty_flags = 0;
}

/// Timed variant of run_all_systems — wraps each system call with profiling
/// and stores results in world.system_times (microseconds per system).
#[cfg(target_family = "wasm")]
pub fn run_all_systems_timed(world: &mut GameWorld) {
    run_all_systems(world);
}

#[cfg(not(target_family = "wasm"))]
pub fn run_all_systems_timed(world: &mut GameWorld) {
    let mut times = IndexMap::new();

    macro_rules! timed {
        ($name:expr, $call:expr) => {{
            let start = Instant::now();
            $call;
            times.insert($name.to_string(), start.elapsed().as_micros() as u64);
        }};
    }

    macro_rules! timed_if {
        ($name:expr, $cond:expr, $call:expr) => {{
            if $cond {
                let start = Instant::now();
                $call;
                times.insert($name.to_string(), start.elapsed().as_micros() as u64);
            } else {
                times.insert($name.to_string(), 0);
            }
        }};
    }

    // Always-run systems
    timed!("construction", construction::run(world));
    timed!("orbital", orbital::run(world));
    timed!("satellite_network", satellite_network::run(world));
    timed!("maintenance", maintenance::run(world));
    timed!("population", population::run(world));
    timed!("coverage", coverage::run(world));
    timed!("demand", demand::run(world));
    timed!("routing", routing::run(world));
    timed!("utilization", utilization::run(world));

    // Conditionally-run systems
    timed_if!("spectrum", should_run_spectrum(world), spectrum::run(world));
    timed!("ftth", ftth::run(world));
    timed_if!(
        "manufacturing",
        should_run_manufacturing(world),
        manufacturing::run(world)
    );
    timed_if!("launch", should_run_launch(world), launch::run(world));
    timed!("terminal_distribution", terminal_distribution::run(world));
    timed!("satellite_revenue", satellite_revenue::run(world));
    timed!("revenue", revenue::run(world));
    timed!("cost", cost::run(world));
    timed!("finance", finance::run(world));
    timed!("contract", contract::run(world));
    timed!("ai", ai::run(world));
    timed_if!("debris", should_run_debris(world), debris::run(world));
    timed_if!(
        "servicing",
        should_run_servicing(world),
        servicing::run(world)
    );
    timed!("regulation", regulation::run(world));
    timed!("research", research::run(world));
    timed_if!("patent", should_run_patent(world), patent::run(world));
    timed!("market", market::run(world));
    timed_if!("auction", should_run_auction(world), auction::run(world));
    timed_if!(
        "covert_ops",
        should_run_covert_ops(world),
        covert_ops::run(world)
    );
    timed_if!("lobbying", should_run_lobbying(world), lobbying::run(world));
    timed_if!("alliance", should_run_alliance(world), alliance::run(world));
    timed_if!("legal", should_run_legal(world), legal::run(world));
    timed!("grants", grants::run(world));
    timed!("achievement", achievement::run(world));
    timed!("stock_market", stock_market::run(world));
    timed!(
        "resolve_spectrum_auctions",
        world.resolve_spectrum_auctions()
    );

    world.system_times = times;

    // Reset dirty flags for next tick
    world.dirty_flags = 0;
}
