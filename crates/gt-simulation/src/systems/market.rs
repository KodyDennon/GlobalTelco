use crate::world::GameWorld;

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

        // Update city infrastructure satisfaction based on actual capacity vs demand
        let city_ids: Vec<u64> = world
            .regions
            .get(&region_id)
            .map(|r| r.city_ids.clone())
            .unwrap_or_default();

        for &city_id in &city_ids {
            let city_cell = match world.cities.get(&city_id) {
                Some(c) => c.cell_index,
                None => continue,
            };

            let city_capacity: f64 = world
                .infra_nodes
                .values()
                .filter(|n| n.cell_index == city_cell)
                .map(|n| n.max_throughput)
                .sum();

            let city_demand = world
                .cities
                .get(&city_id)
                .map(|c| c.telecom_demand)
                .unwrap_or(0.0);

            let base_satisfaction = if city_demand > 0.0 {
                (city_capacity / city_demand).min(1.0)
            } else {
                0.0
            };

            if let Some(city) = world.cities.get_mut(&city_id) {
                city.infrastructure_satisfaction = (base_satisfaction + competition_bonus).min(1.0);
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
}
