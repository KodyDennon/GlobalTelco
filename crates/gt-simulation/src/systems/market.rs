use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Market updates every 20 ticks to avoid excessive recalculation
    if !tick.is_multiple_of(20) {
        return;
    }

    // 1. Calculate regional supply/demand equilibrium
    let mut region_ids: Vec<u64> = world.regions.keys().copied().collect();
    region_ids.sort_unstable();

    for &region_id in &region_ids {
        let region_cells = match world.regions.get(&region_id) {
            Some(r) => r.cells.clone(),
            None => continue,
        };

        // Sum total demand from cities in this region
        let total_demand: f64 = world
            .cities
            .values()
            .filter(|c| world.cell_to_region.get(&c.cell_index).copied() == Some(region_id))
            .map(|c| c.telecom_demand)
            .sum();

        // Sum total supply from infrastructure in this region
        let total_supply: f64 = world
            .infra_nodes
            .values()
            .filter(|n| region_cells.contains(&n.cell_index))
            .map(|n| n.max_throughput)
            .sum();

        // Update region demand component
        if let Some(demand) = world.demands.get_mut(&region_id) {
            demand.current_demand = total_demand;
            demand.satisfaction = if total_demand > 0.0 {
                (total_supply / total_demand).min(1.0)
            } else {
                1.0
            };
        }

        // Adjust regional pricing based on supply/demand ratio
        // High demand + low supply = higher prices (good for existing providers)
        // High supply + low demand = lower prices (competitive pressure)
        let supply_demand_ratio = if total_demand > 0.0 {
            total_supply / total_demand
        } else {
            1.0
        };

        // Update tax rate slightly toward equilibrium
        // Regions with high satisfaction get slightly lower taxes (pro-business)
        // Regions with low satisfaction increase taxes to fund public infrastructure
        if let Some(region) = world.regions.get_mut(&region_id) {
            if supply_demand_ratio > 1.5 {
                // Oversupply → slight tax reduction to attract more business
                region.tax_rate = (region.tax_rate - 0.001).max(0.05);
            } else if supply_demand_ratio < 0.5 {
                // Undersupply → slight tax increase
                region.tax_rate = (region.tax_rate + 0.001).min(0.45);
            }
        }
    }

    // 2. Update global interest rates based on economic health
    // Count profitable vs unprofitable corps
    let mut profitable_count = 0u32;
    let mut total_corps = 0u32;

    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();
    for &corp_id in &corp_ids {
        if let Some(fin) = world.financials.get(&corp_id) {
            total_corps += 1;
            if fin.revenue_per_tick > fin.cost_per_tick {
                profitable_count += 1;
            }
        }
    }

    // Economic health affects AI willingness to invest
    let economic_health = if total_corps > 0 {
        profitable_count as f64 / total_corps as f64
    } else {
        0.5
    };

    // 3. Update market competition dynamics per region
    // Count how many corps operate in each region and adjust infrastructure satisfaction
    for &region_id in &region_ids {
        let region_cells = match world.regions.get(&region_id) {
            Some(r) => r.cells.clone(),
            None => continue,
        };

        // Count unique corps operating in this region
        let corps_in_region: std::collections::HashSet<u64> = world
            .infra_nodes
            .values()
            .filter(|n| region_cells.contains(&n.cell_index))
            .map(|n| n.owner)
            .collect();

        let competition_level = corps_in_region.len();

        // More competition → higher infrastructure satisfaction for cities
        let competition_bonus = match competition_level {
            0 => 0.0,
            1 => 0.0,
            2 => 0.1,
            3 => 0.2,
            _ => 0.3,
        };

        // Apply competition bonus to existing satisfaction (calculated by demand system)
        // Don't recalculate satisfaction — just add the competition bonus
        let city_ids: Vec<u64> = world
            .regions
            .get(&region_id)
            .map(|r| r.city_ids.clone())
            .unwrap_or_default();

        for &city_id in &city_ids {
            if let Some(city) = world.cities.get_mut(&city_id) {
                // Add competition bonus on top of existing satisfaction from demand system
                city.infrastructure_satisfaction =
                    (city.infrastructure_satisfaction + competition_bonus).min(1.0);
            }
        }
    }

    // 4. Emit market report event periodically (every 100 ticks)
    if tick.is_multiple_of(100) {
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::MarketUpdate {
                economic_health,
                active_corporations: total_corps,
            },
        );
    }

    // 5. Check for underserved markets and spawn new AI corporations
    check_market_gaps(world);
}

// ─── Dynamic AI Spawning ────────────────────────────────────────────────────

/// Check for underserved regions and spawn new AI corporations to fill market gaps.
/// Only runs every 100 ticks. At most 1 new AI corp per check.
fn check_market_gaps(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only check every 100 ticks
    if !tick.is_multiple_of(100) || tick == 0 {
        return;
    }

    // Count current AI corporations
    let ai_corp_count = world
        .ai_states
        .len() as u32;

    let max_ai = world.config().max_ai_corporations;
    if ai_corp_count >= max_ai {
        return;
    }

    // Find underserved regions: population > 10000 and < 2 corporations serving them
    let mut region_ids: Vec<EntityId> = world.regions.keys().copied().collect();
    region_ids.sort_unstable();

    let mut underserved_regions: Vec<EntityId> = Vec::new();

    for &region_id in &region_ids {
        let region = match world.regions.get(&region_id) {
            Some(r) => r,
            None => continue,
        };

        // Calculate total population in this region
        let region_population: u64 = region
            .city_ids
            .iter()
            .filter_map(|&city_id| world.cities.get(&city_id))
            .map(|city| city.population)
            .sum();

        if region_population < 10_000 {
            continue;
        }

        // Count unique corporations with nodes in this region
        let region_cells = &region.cells;
        let corps_in_region: std::collections::HashSet<EntityId> = world
            .infra_nodes
            .values()
            .filter(|n| region_cells.contains(&n.cell_index))
            .map(|n| n.owner)
            .collect();

        if corps_in_region.len() < 2 {
            underserved_regions.push(region_id);
        }
    }

    if underserved_regions.is_empty() {
        return;
    }

    // Pick a deterministic "random" underserved region
    let region_index = ((tick.wrapping_mul(7919)) % underserved_regions.len() as u64) as usize;
    let _target_region = underserved_regions[region_index];

    // Name pool for dynamically spawned AI corps
    let spawn_names = [
        "Apex Telecom",
        "Horizon Networks",
        "Pinnacle Connect",
        "Nova Communications",
        "Vertex Global",
        "Summit Wireless",
        "Citadel Net",
        "Vanguard Telecom",
    ];

    // Pick a name not already in use
    let existing_names: std::collections::HashSet<&str> = world
        .corporations
        .values()
        .map(|c| c.name.as_str())
        .collect();

    let name = spawn_names
        .iter()
        .find(|&&n| !existing_names.contains(n))
        .unwrap_or(&"NewCo Telecom");

    // Pick archetype based on tick for deterministic variety
    let archetypes = [
        AIArchetype::AggressiveExpander,
        AIArchetype::DefensiveConsolidator,
        AIArchetype::TechInnovator,
        AIArchetype::BudgetOperator,
    ];
    let archetype_index = ((tick.wrapping_mul(ai_corp_count as u64 + 1)) % 4) as usize;
    let archetype = archetypes[archetype_index];

    // Spawn the new AI corporation
    let ai_id = world.allocate_entity();

    world.corporations.insert(ai_id, Corporation::new(*name, false));
    world.financials.insert(
        ai_id,
        Financial {
            cash: 500_000,
            revenue_per_tick: 0,
            cost_per_tick: 0,
            debt: 0,
        },
    );
    world.ai_states.insert(ai_id, AiState::new(archetype));
    world.policies.insert(ai_id, Policy::default());
    world.workforces.insert(
        ai_id,
        Workforce {
            employee_count: 10,
            skill_level: 0.5,
            morale: 0.7,
            salary_per_tick: 1000,
            maintenance_crew_count: 1,
        },
    );

    world.event_queue.push(
        tick,
        gt_common::events::GameEvent::CorporationFounded {
            entity: ai_id,
            name: name.to_string(),
        },
    );
}
